# `devnet/` — el gate 1.4 contra una cadena de verdad

Los 34 tests de `../scripts/gate.sh` corren el programa **nativo, en proceso**. Eso prueba
la lógica con el modelo de cuentas real, pero no prueba que el programa exista como
artefacto: un programa que solo corre nativo no es un programa. Acá se compila a SBF, se
despliega y se corren los dos escenarios del gate sobre un validador.

```
program-keypair.json   la direccion del programa. **Es de prueba, no protege nada.**
payer/worker/          los cuatro actores del escenario, tambien de prueba
challenger/arbiter.json
build.sh               cargo build-sbf -> deploy/pod_escrow.so
setup.sh               keypairs y configuracion
airdrop.sh             pide saldo al faucet de devnet (limitado por IP)
localnet.sh            valida el ciclo completo contra un validador local
ix.mjs                 las 7 instrucciones, reimplementadas desde SPEC-PROGRAM.md §4
gate.mjs               los dos escenarios del gate 1.4
```

**Los keypairs de este directorio son públicos a propósito.** Están en el repo para que la
evidencia sea reproducible sin ceremonia. No tienen valor y no deben tenerlo nunca: si algo
de esto llegara a mainnet, se generan claves nuevas y estas se tiran.

## Por qué el cliente está en JavaScript y no reusa el código del programa

`ix.mjs` arma las instrucciones **leyendo el documento**, sin importar una línea de Rust. Es
la misma disciplina que `pod/spec`, donde dos implementaciones independientes tienen que
producir el mismo `spec_hash`: si SPEC-PROGRAM.md §4 fuera ambiguo, el cliente y el programa
no se entenderían y el gate fallaría. Reusar el encoder del programa haría pasar el test
sin decir nada sobre el documento.

## Validador local, no devnet

`..._FASE1.md` §5 pide el gate **en devnet**. Se corre contra un validador local, y la
diferencia hay que decirla con precisión en vez de esconderla.

**Igual que devnet:** el mismo runtime de Agave, la misma VM de SBF ejecutando el mismo
`.so`, el mismo presupuesto de cómputo, las mismas fees, el mismo modelo de rent y un reloj
real (no un sysvar escrito a mano como en los tests en proceso).

**Distinto de devnet:** la red no es compartida. No hay otras transacciones compitiendo, ni
contención de blockhash, ni un RPC público con sus límites. Nada de eso es parte de lo que
el gate afirma —quién pierde el bond y quién pierde el depósito— pero tampoco está probado.

**Por qué no devnet:** el faucet público limita por IP y no soltó los ~2 SOL que cuesta el
despliegue. `airdrop.sh` queda escrito y reintentable; el día que el faucet coopere,
`node gate.mjs https://api.devnet.solana.com` cierra el tramo sin tocar una línea de código.
