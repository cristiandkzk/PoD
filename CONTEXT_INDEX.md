# CONTEXT_INDEX

Tabla de lookup del repo. La lee la skill `context-retrieval` en la fase de Discovery
para saltear el grep inicial. **La escribe el agente**, incrementalmente, a medida que
descubre áreas nuevas — no es documentación mantenida a mano.

> # ⬛ PROYECTO ARCHIVADO — 2026-08-13
>
> Se archivó en el checkpoint 1.4, por el **criterio de kill de demanda** del plan maestro §10.
> No quedó nada roto ni a medio hacer: las cuatro subfases construidas pasan sus gates. Lo que
> falló no fue técnico.
>
> **Empezá por [`ARCHIVO.md`](ARCHIVO.md)** — qué se probó, qué se midió, por qué se archivó, y
> el filtro que decide si alguna otra clase de trabajo lo revive.

Estado del repo: **Fase 1, subfases 1.1 a 1.4 construidas y verificadas** (spec canónica, runner
determinístico, escrow on-chain y verificación optimista). La 1.5 quedó **rediseñada y sin
empezar**; la 1.6 nunca se abrió. La Fase 0 no corrió: E0.1 y E0.2 siguen pendientes.

Este índice sigue siendo válido como mapa del código: nada se movió al archivar.

## Qué hay hoy

| Tema | Dónde | Qué contiene |
|---|---|---|
| **Concepto — Proof of Delivery** | [`2026-08-12T04-08-12_evolucion-de-blockchain.md`](2026-08-12T04-08-12_evolucion-de-blockchain.md) | El debate (posturas A/B/C), el punto de choque, y la tesis: "trabajo pedido, pagado y entregado" como hecho criptográficamente verificable, y única fuente de emisión |
| **Plan de implementación** | [`..._PLAN.md`](2026-08-12T04-08-12_evolucion-de-blockchain_PLAN.md) | Fases 0–4, gates, decisiones técnicas, riesgos, criterios de kill |
| **Fase 1 dividida en subfases** | [`..._FASE1.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) | Subfases 1.1 spec · 1.2 runner · 1.3 escrow · 1.4 optimista · 1.5 `WorkSettled` · 1.6 e2e, más track de demanda. **Protocolo de checkpoint: se para y se pregunta entre subfases** |
| **Fase 2 dividida en subfases** | [`..._FASE2.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE2.md) | Subfases 2.1 `W(t)` canónico · 2.2 grafo de financiamiento · 2.3 calibración del detector · 2.4 umbral y medición · 2.5 salud del mecanismo. Precondiciones de entrada, bifurcación por E0.2, y la quinta respuesta de checkpoint: **Indeterminado** |
| **Subfase 1.1 — spec canónica** | [`pod/spec/SPEC.md`](pod/spec/SPEC.md) | Normativa del formato: canonicalización, esquema cerrado del `WorkOrder`, rejillas de barrido, códigos de error, dominio del hash. Es el árbitro entre las dos implementaciones. §10 explica por qué §6 se rehizo |
| **Clase de trabajo — de dónde sale** | [`memebot/backtest.mjs`](../memebot/backtest.mjs) | El backtester real que `backtest.sweep.v1` modela: 3 estrategias, barrido de ~1000 combinaciones sobre `data/ticks-*.jsonl`, validación train/test por día |
| **Subfase 1.2 — runner** | [`pod/prover/SPEC-RUNNER.md`](pod/prover/SPEC-RUNNER.md) | Normativa de ejecución: modelo numérico, carga del dataset, simulación, barrido, `output_hash`, recibo firmado y replay |
| **Subfases 1.3 y 1.4 — escrow y verificación** | [`pod/program/SPEC-PROGRAM.md`](pod/program/SPEC-PROGRAM.md) | Normativa del dinero: PDA `Order`, layout de 304 bytes, siete instrucciones, invariantes I1–I10, 21 errores. **§7 es la declaración del árbitro** (opción (a) de FASE1 §5) con su plan de reemplazo; §8 la interfaz ZK declarada y no implementada; §10 lo que se decide **no** decidir |
| **Escalera de verificación** (Niveles 0/1/2, default optimista) | `..._PLAN.md` §0 y §7 | Por qué el default es Nivel 1 y no ZK |
| **Fase 0 — KPIs de falsación** | `..._PLAN.md` §2 | E0.1 overhead de verificación · E0.2 ventana de `k` |
| **Máquina de estados `WorkOrder`** | `..._PLAN.md` §3 (resumen) · `..._FASE1.md` §4–§5 (implementación) | CREADA → ACEPTADA → ENTREGADA → LIQUIDADA / FALLIDA |
| **Fórmula de emisión** `E(t) = min(curva(t), k·W(t))` | `..._PLAN.md` §5 | Solo si pasa el gate de E0.2 |
| **Criterios de kill explícitos** | `..._PLAN.md` §10 | Umbrales fijados antes del costo hundido |
| **Sesión origen** | [`2026-08-12T04-08-12_evolucion-de-blockchain/`](2026-08-12T04-08-12_evolucion-de-blockchain/SESSION_SUMMARY.md) | Transcripción (`SESSION_SUMMARY.md`), crudo (`raw/session.json`), diagramas ASCII (`snippets/*.txt`) |

## Herramientas del repo

| Tema | Dónde |
|---|---|
| Escalera de costo (diseño: qué maquinaria merece el problema) | [`.claude/skills/escalera-de-costo/SKILL.md`](.claude/skills/escalera-de-costo/SKILL.md) |
| Auditoría de capas de ahorro de tokens + KPI | [`.claude/skills/auditoria-token-arch/SKILL.md`](.claude/skills/auditoria-token-arch/SKILL.md) |
| Script del KPI (Nivel 0, sin dependencias) | [`.claude/skills/auditoria-token-arch/scripts/kpi_ahorro.py`](.claude/skills/auditoria-token-arch/scripts/kpi_ahorro.py) |
| Permisos del proyecto (grep-first sin prompts) | [`.claude/settings.json`](.claude/settings.json) |
| Búsqueda de código (global) | `~/.claude/skills/context-retrieval/SKILL.md` |

## Dónde va el código cuando exista

Layout propuesto en `..._PLAN.md` §3. Anotar acá la ruta real en cuanto se cree cada pieza.

| Módulo | Ruta prevista | Responsabilidad |
|---|---|---|
| Spec | [`pod/spec/`](pod/spec/README.md) | **Existe.** Formato canónico + `spec_hash`. Normativa en [`pod/spec/SPEC.md`](pod/spec/SPEC.md); dos implementaciones (`python/`, `rust/`) y vectores congelados |
| Contrato | [`pod/program/`](pod/program/README.md) | **Existe.** Escrow, bond, timeouts, `deliver`/`challenge`/`resolve`/`settle` en Solana **nativo** (no Anchor — el README dice por qué). Cliente de cadena en `devnet/`, reimplementado desde el documento |
| Prover | [`pod/prover/`](pod/prover/README.md) | **Existe.** Runner determinístico + recibo firmado Ed25519 + replay. Nivel 0 completo; Niveles 1 y 2 son 1.4 |
| Indexer | `pod/indexer/` | `WorkSettled` → base de datos → `W(t)`. Sube de *smoke* (1.5) a canónico y reproducible (2.1) |
| Grafo anti-sybil | `pod/indexer/graph/` | Grafo de financiamiento `payer`↔`worker`, Nivel 0, sin modelo (2.2) |
| SDK de agentes | `pod/agent-sdk/` | `request_work()` / `fulfill()` / `claim()` |
| Simulaciones | `pod/sim/` | KPI 2 (`k_window.py`), red team |

**Primer archivo del plan (§9):** `sim/k_window.py` — modelo honesto vs. auto-pagador,
responde el gate más caro. Todavía no existe.

**Verificación final al archivar — 2026-08-13, todos en verde:**

| Gate | Comando | Resultado |
|---|---|---|
| 1.1 spec | `python pod/spec/scripts/gate.py` | los cuatro gates pasan |
| 1.2 runner | `python pod/prover/scripts/gate.py` | 3/3 pedidos en las 3 plataformas de `PLATFORMS.tsv` |
| 1.3 + 1.4 | `bash pod/program/scripts/gate.sh` *(Linux o WSL)* | **34 tests**, exit 0 |
| 1.4 en cadena | `bash pod/program/devnet/localnet.sh` | los dos escenarios pasan — ver [`CADENA.txt`](pod/program/CADENA.txt) |

`pod/program` necesita **Linux o WSL**: el árbol de `solana-program-test` no compila en Windows
porque arrastra openssl vendorizado, que pide perl y nmake.

**Lo que quedó abierto**, todo documentado y nada a medio construir: la subfase 1.5 rediseñada
sin empezar ([`..._FASE1.md`](2026-08-12T04-08-12_evolucion-de-blockchain_FASE1.md) §6), la
disponibilidad de datos (§6.3), los dos roles impagos —árbitro y challenger— y el track D de
demanda, que es el que disparó el archivo. El detalle está en [`ARCHIVO.md`](ARCHIVO.md) §6.

## Términos → dónde buscar

`spec_hash`, `canonicalización`, `WorkOrder`, `códigos de error`, `E_*`, `vectores`, `dominio del hash` → [`pod/spec/SPEC.md`](pod/spec/SPEC.md) ·
`WorkSettled`, `bond`, `challenge window`, `escrow` → `..._FASE1.md` (detalle) y `..._PLAN.md` §3 (resumen) ·
`subfase`, `checkpoint`, `gate intermedio`, `track de demanda`, `canonicalización`, `replay` → `..._FASE1.md` ·
`output_hash`, `recibo`, `replay`, `determinismo`, `Ed25519`, `barrido`, `fidelidad` → [`pod/prover/SPEC-RUNNER.md`](pod/prover/SPEC-RUNNER.md) ·
`escrow`, `PDA`, `rent`, `bond`, `cancel_expired`, `conservación de balances`, `slashing`, `autotrato` → [`pod/program/SPEC-PROGRAM.md`](pod/program/SPEC-PROGRAM.md) ·
`challenge`, `árbitro`, `deliver`, `settle`, `veredicto`, `depósito`, `ventana`, `ZK`, `proof_mode`, `disponibilidad de datos` → [`pod/program/SPEC-PROGRAM.md`](pod/program/SPEC-PROGRAM.md) §7–§10 ·
`W(t)`, `época`, `wash work`, `sybil`, `grafo de financiamiento`, `hubs`, `calibración`, `indexer canónico` → `..._FASE2.md` (detalle) y `..._PLAN.md` §4 (resumen) ·
`k`, `emisión`, `red team` → `..._PLAN.md` §2 (E0.2), §5 ·
`Proof of Delivery`, `PoD`, `emisión`, `Postura A/B/C` → documento de concepto
