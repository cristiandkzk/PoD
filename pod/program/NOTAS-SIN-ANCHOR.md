# Siete cosas que me rompieron al escribir un programa de Solana sin Anchor

Casi toda la documentación de Solana asume Anchor. Cuando escribís nativo, los problemas que
aparecen no están en ningún tutorial — están en issues de GitHub cerrados sin respuesta y en
mensajes de error que no dicen lo que pasa.

Estas son las siete que me costaron tiempo, con la causa y el arreglo. Todas salieron de escribir
[un escrow con verificación optimista](README.md): 7 instrucciones, cuenta de 304 bytes, 34 tests.
El código está entero en este repo.

---

## 1. `Clock::get()` no funciona fuera de SBF, y el error no lo dice

**Síntoma:** los tests pasan la lógica y fallan con `UnsupportedSysvar` al llegar al primer
`Clock::get()`.

**Causa:** `Clock::get()` y `Rent::get()` usan syscalls que solo existen dentro de la VM de SBF.
En `solana-program-test` con `processor!` el programa corre **nativo, en proceso**, y el shim de
sysvars devuelve `UNSUPPORTED_SYSVAR`. El error no menciona ni el sysvar ni por qué.

**Arreglo:** pasar los sysvars como cuentas y leerlos desde ahí.

```rust
use solana_program::sysvar::{self, clock::Clock, rent::Rent, SysvarSerialize};

fn check_sysvars(rent: Option<&AccountInfo>, clock: &AccountInfo) -> Result<(), ProgramError> {
    if let Some(r) = rent {
        if r.key != &sysvar::rent::ID { return Err(PodError::SysvarInvalido.into()); }
    }
    if clock.key != &sysvar::clock::ID { return Err(PodError::SysvarInvalido.into()); }
    Ok(())
}

let ahora = Clock::from_account_info(clock_ai)?.unix_timestamp;
```

El chequeo explícito de `key` no es opcional: sin él, cualquiera pasa una cuenta arbitraria donde
va el `Clock`. Anchor lo haría con un constraint; acá lo escribís vos, y **eso es exactamente lo
que querés poder testear**.

## 2. El `.so` compila bien y el validador lo rechaza

**Síntoma:** `cargo-build-sbf` termina sin errores, `solana program deploy` falla con
`Detected sbpf_version ... not enabled`.

**Causa:** agave activó `disable_sbpf_v0_execution`. El default de `cargo-build-sbf` sigue siendo
v0, así que producís un binario que la cadena ya no ejecuta.

**Arreglo:**

```bash
cargo-build-sbf --arch v3
```

Nada en el output de la compilación sugiere que haga falta.

## 3. `solana-program 4.x` se quedó sin `system_instruction`

**Síntoma:** `unresolved import solana_program::system_instruction`.

**Causa:** en la 4.x sacaron el módulo del system program a un crate aparte.

**Arreglo:**

```toml
solana-system-interface = { version = "3", features = ["bincode"] }
```

Y de paso: `AccountInfo::realloc` pasó a llamarse **`resize`**.

## 4. `solana-program-test` te fija la versión de `solana-program`

**Síntoma:** errores de tipos incompatibles entre tu crate y el harness de tests, con dos copias
de `solana-program` en el árbol de dependencias.

**Causa:** `solana-program-test 4.2` depende de una versión concreta de `solana-program`. Si vos
pineás `4.1`, cargo compila las dos y los tipos no son el mismo tipo.

**Arreglo:** aflojar la restricción y dejar que cargo resuelva:

```toml
solana-program = "4"
```

## 5. `solana-program-test` no linkea si no le pedís la feature inestable

**Síntoma:** símbolos no encontrados al linkear los tests.

**Arreglo:**

```toml
[dev-dependencies]
solana-program-test = { version = "4.2", features = ["agave-unstable-api"] }
```

El nombre de la feature es una advertencia honesta: esa API se mueve.

## 6. En Windows no compila, y no es culpa tuya

**Síntoma:** el build de los tests falla pidiendo `perl` y `nmake`.

**Causa:** `solana-program-test` arrastra `openssl` vendorizado, cuyo build script necesita la
toolchain de C de Windows. No hay flag que lo evite.

**Arreglo:** WSL. Y si lo invocás desde Git Bash, hace falta frenar la traducción de rutas:

```bash
MSYS_NO_PATHCONV=1 wsl -d Ubuntu-24.04 -- bash /mnt/c/.../scripts/gate.sh
```

Sin `MSYS_NO_PATHCONV`, Git Bash convierte `/mnt/c/...` en `C:/Program Files/Git/mnt/c/...` y el
error que ves no tiene nada que ver con la causa.

## 7. Vaciar una cuenta no la cierra

Esta es la que importa de verdad, porque las seis anteriores te hacen perder una tarde y esta te
hace perder plata.

**Síntoma:** ninguno. Los tests pasan. La cuenta "cerrada" se puede revivir.

**Causa:** poner los lamports en cero no borra nada. Los datos siguen ahí y el dueño sigue siendo
tu programa, así que alguien puede mandarle lamports de vuelta y la cuenta reaparece **con su
estado anterior intacto** — una orden ya liquidada vuelve a estar viva.

**Arreglo, los tres pasos:**

```rust
// 1. los lamports ya salieron y tienen que haber salido TODOS
if order.lamports() != 0 { return Err(PodError::CierreIncompleto.into()); }

// 2. borrar los datos
order.try_borrow_mut_data()?.fill(0);

// 3. soltar la cuenta: tamaño cero y devolverla al system program
order.resize(0)?;
order.assign(&system_program::ID);
```

El chequeo del paso 1 es a propósito: si quedaron lamports, la transacción **falla** en vez de
dejarlos encerrados para siempre en una cuenta que nadie puede volver a tocar.

Y el test que lo fija:

```rust
#[tokio::test]
async fn i7_una_orden_cerrada_no_revive() { /* ... */ }
```

---

## Lo que aprendí sobre nativo vs Anchor

No es "nativo es mejor". Anchor te resuelve mucho y para la mayoría de los programas conviene.

La diferencia que sí importa es **qué termina probando tu suite de tests**. Con Anchor, buena
parte de la seguridad de cuentas vive en macros de constraints, así que tus tests prueban en gran
medida que Anchor cumple lo que promete — cosa que ya sabés. Escribiendo nativo, cada chequeo
—dueño, firmante, PDA derivado, discriminante, longitud exacta del buffer, sysvar correcto— es
una línea tuya, y podés escribir el test que la rompe.

Para siete instrucciones, ese intercambio valió la pena. Para cuarenta, probablemente no.

---

*Todo el código, los 34 tests y la evidencia de ejecución en cadena están en
[este repo](https://github.com/cristiandkzk/PoD). Es un proyecto archivado — el
[porqué](../../ARCHIVO.md) también está escrito.*
