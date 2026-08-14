//! Validación del esquema cerrado del WorkOrder — SPEC.md §6 y §7.1 pasos 2 y 3.

use std::collections::BTreeMap;

use crate::error::SpecError;
use crate::json::Value;

const SCALE_DIGITS: usize = 18;
const I64_MAX: i64 = i64::MAX;

/// Cota superior de los decimales sin tope real (18 dígitos enteros, SPEC §3.4).
const UNLIMITED_DEC: &str = "999999999999999999";

const METRICS: [&str; 6] = [
    "gross_loss_lamports",
    "gross_win_lamports",
    "n_trades",
    "net_lamports",
    "unclosed",
    "wins",
];

// Listas de claves declaradas. DEBEN estar en orden ascendente por bytes: el recorrido de
// SPEC §7.1.2 las fusiona con las claves presentes asumiendo que ambas vienen ordenadas.
const ROOT_KEYS: [&str; 9] = [
    "class",
    "deadline",
    "inputs",
    "limits",
    "output_shape",
    "payment",
    "proof_mode",
    "runner",
    "schema_version",
];
const INPUTS_KEYS: [&str; 6] = [
    "costs",
    "dataset",
    "exit_policy",
    "portfolio",
    "split",
    "strategy",
];
const DATASET_KEYS: [&str; 7] = [
    "days",
    "first_unix_ms",
    "format",
    "hash",
    "last_unix_ms",
    "records",
    "tokens",
];
const SURVIVOR_KEYS: [&str; 6] = [
    "max_age_min",
    "min_age_min",
    "min_buyers",
    "min_growth",
    "min_mcap",
    "min_ratio",
];
const SNIPER_KEYS: [&str; 4] = [
    "max_dev_lamports",
    "min_dev_lamports",
    "panic_lamports",
    "stall_s",
];
const GRADUACION_KEYS: [&str; 3] = ["abort_bps", "dip_bps", "timeout_min"];
const EXIT_POLICY_KEYS: [&str; 8] = [
    "hard_stop_bps",
    "time_stop_min",
    "tp_mult",
    "tp_sell_bps",
    "trail_always",
    "trail_arm_bps",
    "trail_bps",
    "trail_sell_bps",
];
const PORTFOLIO_KEYS: [&str; 2] = ["max_open_positions", "notional_lamports"];
const COSTS_KEYS: [&str; 2] = ["fee_lamports_per_tx", "slippage_bps_per_side"];
const OUTPUT_SHAPE_KEYS: [&str; 4] = ["format", "metrics", "rounding", "top_n"];
const RUNNER_KEYS: [&str; 5] = [
    "commit",
    "entrypoint",
    "image_digest",
    "image_ref",
    "toolchain",
];
const LIMITS_KEYS: [&str; 3] = ["cpu_count", "memory_bytes", "wall_time_s"];
const DEADLINE_KEYS: [&str; 2] = ["accept_by_unix_s", "deliver_within_s"];
const PAYMENT_KEYS: [&str; 4] = [
    "amount_base_units",
    "bond_base_units",
    "mint",
    "mint_decimals",
];

// ------------------------------------------------------------------------- primitivas

fn as_object<'a>(v: &'a Value, path: &str) -> Result<&'a BTreeMap<String, Value>, SpecError> {
    match v {
        Value::Object(m) => Ok(m),
        _ => Err(SpecError::new("E_TYPE", path)),
    }
}

/// Una sola pasada sobre la unión de claves declaradas y presentes, en orden canónico
/// (SPEC §7.1.2). Las dos secuencias ya están ordenadas, así que es una fusión.
fn walk<F>(v: &Value, path: &str, declared: &[&str], mut visit: F) -> Result<(), SpecError>
where
    F: FnMut(&str, &Value, &str) -> Result<(), SpecError>,
{
    let map = as_object(v, path)?;
    let mut present = map.iter().peekable();
    let mut di = 0usize;
    loop {
        let d = declared.get(di).copied();
        let p = present.peek().map(|(k, _)| k.as_str());
        match (d, p) {
            (None, None) => return Ok(()),
            (Some(d), None) => return Err(SpecError::new("E_MISSING_FIELD", format!("{path}.{d}"))),
            (None, Some(p)) => return Err(SpecError::new("E_UNKNOWN_FIELD", format!("{path}.{p}"))),
            (Some(d), Some(p)) if d < p => {
                return Err(SpecError::new("E_MISSING_FIELD", format!("{path}.{d}")))
            }
            (Some(_), Some(p)) if p < declared[di] => {
                return Err(SpecError::new("E_UNKNOWN_FIELD", format!("{path}.{p}")))
            }
            (Some(d), Some(_)) => {
                let (_, value) = present.next().expect("peek dijo que hay");
                visit(d, value, &format!("{path}.{d}"))?;
                di += 1;
            }
        }
    }
}

/// Unión etiquetada: `kind` se resuelve antes de recorrer el resto (SPEC §6.11).
fn tag<'a>(v: &'a Value, path: &str, allowed: &[&str]) -> Result<&'a str, SpecError> {
    let map = as_object(v, path)?;
    let kind = map
        .get("kind")
        .ok_or_else(|| SpecError::new("E_MISSING_FIELD", format!("{path}.kind")))?;
    let kind = match kind {
        Value::Str(s) => s.as_str(),
        _ => return Err(SpecError::new("E_TYPE", format!("{path}.kind"))),
    };
    if !allowed.contains(&kind) {
        return Err(SpecError::new("E_ENUM", format!("{path}.kind")));
    }
    Ok(kind)
}

fn v_int(v: &Value, path: &str, lo: i64, hi: i64) -> Result<i64, SpecError> {
    match v {
        Value::Int(n) if *n >= lo && *n <= hi => Ok(*n),
        Value::Int(_) => Err(SpecError::new("E_INT_RANGE", path)),
        _ => Err(SpecError::new("E_TYPE", path)),
    }
}

fn v_bool(v: &Value, path: &str) -> Result<(), SpecError> {
    match v {
        Value::Bool(_) => Ok(()),
        _ => Err(SpecError::new("E_TYPE", path)),
    }
}

fn v_str<'a>(v: &'a Value, path: &str) -> Result<&'a str, SpecError> {
    match v {
        Value::Str(s) => Ok(s.as_str()),
        _ => Err(SpecError::new("E_TYPE", path)),
    }
}

fn v_enum(v: &Value, path: &str, allowed: &[&str]) -> Result<(), SpecError> {
    let s = v_str(v, path)?;
    if allowed.contains(&s) {
        Ok(())
    } else {
        Err(SpecError::new("E_ENUM", path))
    }
}

fn v_shape(v: &Value, path: &str, ok: fn(&str) -> bool) -> Result<(), SpecError> {
    let s = v_str(v, path)?;
    if ok(s) {
        Ok(())
    } else {
        Err(SpecError::new("E_STRING_FORMAT", path))
    }
}

fn v_array<'a>(v: &'a Value, path: &str) -> Result<&'a Vec<Value>, SpecError> {
    match v {
        Value::Array(items) => Ok(items),
        _ => Err(SpecError::new("E_TYPE", path)),
    }
}

// -------------------------------------------------------------------- rejillas §3.5

/// Rejilla de enteros: >=1 elemento, estrictamente ascendente. Devuelve el máximo.
fn grid_int(v: &Value, path: &str, lo: i64, hi: i64) -> Result<i64, SpecError> {
    let items = v_array(v, path)?;
    let mut values = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        values.push(v_int(item, &format!("{path}[{i}]"), lo, hi)?);
    }
    for pair in values.windows(2) {
        if pair[0] >= pair[1] {
            return Err(SpecError::new("E_NOT_SORTED", path));
        }
    }
    Ok(*values.last().expect("array no vacio por SPEC 2"))
}

/// Rejilla de decimales: >=1 elemento, ascendente por valor escalado (SPEC §3.5).
fn grid_decimal(
    v: &Value,
    path: &str,
    lo: i128,
    hi: i128,
    lo_inclusive: bool,
    hi_inclusive: bool,
) -> Result<(), SpecError> {
    let items = v_array(v, path)?;
    let mut values = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let sub = format!("{path}[{i}]");
        let s = v_str(item, &sub)?;
        let scaled = decimal_scaled(s, &sub)?;
        if scaled < lo || (scaled == lo && !lo_inclusive) {
            return Err(SpecError::new("E_DECIMAL_RANGE", sub));
        }
        if scaled > hi || (scaled == hi && !hi_inclusive) {
            return Err(SpecError::new("E_DECIMAL_RANGE", sub));
        }
        values.push(scaled);
    }
    for pair in values.windows(2) {
        if pair[0] >= pair[1] {
            return Err(SpecError::new("E_NOT_SORTED", path));
        }
    }
    Ok(())
}

// -------------------------------------------------------------------------- formas

fn is_lower_hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_sha256_ref(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|h| is_lower_hex(h, 64))
}

fn is_hex40(s: &str) -> bool {
    is_lower_hex(s, 40)
}

fn is_base58(s: &str) -> bool {
    const ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    (32..=44).contains(&s.len()) && s.chars().all(|c| ALPHABET.contains(c))
}

fn is_toolchain(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && b[0].is_ascii_alphanumeric()
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
}

/// Decimal -> entero escalado por 10^18 (SPEC §3.4).
pub fn decimal_scaled(text: &str, path: &str) -> Result<i128, SpecError> {
    let err = || SpecError::new("E_DECIMAL_FORMAT", path);
    let b = text.as_bytes();
    let mut i = 0usize;
    let negative = b.first() == Some(&b'-');
    if negative {
        i = 1;
    }
    if i >= b.len() {
        return Err(err());
    }
    let int_start = i;
    if b[i] == b'0' {
        i += 1;
    } else if b[i].is_ascii_digit() {
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        return Err(err());
    }
    let int_part = &text[int_start..i];

    let mut frac_part = "";
    if i < b.len() {
        if b[i] != b'.' {
            return Err(err());
        }
        i += 1;
        let frac_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i != b.len() {
            return Err(err());
        }
        frac_part = &text[frac_start..i];
        if frac_part.is_empty() || frac_part.as_bytes()[frac_part.len() - 1] == b'0' {
            return Err(err());
        }
    }
    if int_part.len() > 18 || frac_part.len() > 18 {
        return Err(err());
    }

    let mut digits = String::with_capacity(int_part.len() + SCALE_DIGITS);
    digits.push_str(int_part);
    digits.push_str(frac_part);
    for _ in frac_part.len()..SCALE_DIGITS {
        digits.push('0');
    }
    let mut value: i128 = digits.parse().map_err(|_| err())?;
    if negative {
        if value == 0 {
            return Err(err());
        }
        value = -value;
    }
    Ok(value)
}

fn unlimited() -> i128 {
    decimal_scaled(UNLIMITED_DEC, "$").expect("constante valida")
}

// ------------------------------------------------------------------------ subobjetos

fn v_dataset(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &DATASET_KEYS, |key, val, p| match key {
        "days" => v_int(val, p, 1, I64_MAX).map(|_| ()),
        "first_unix_ms" => v_int(val, p, 0, I64_MAX).map(|_| ()),
        "format" => v_enum(val, p, &["ticks.jsonl.v1"]),
        "hash" => v_shape(val, p, is_sha256_ref),
        "last_unix_ms" => v_int(val, p, 0, I64_MAX).map(|_| ()),
        "records" => v_int(val, p, 1, I64_MAX).map(|_| ()),
        "tokens" => v_int(val, p, 1, I64_MAX).map(|_| ()),
        _ => Ok(()),
    })?;
    let map = as_object(v, path)?;
    let first = v_int(&map["first_unix_ms"], path, 0, I64_MAX)?;
    let last = v_int(&map["last_unix_ms"], path, 0, I64_MAX)?;
    if last <= first {
        return Err(SpecError::new("E_CONSTRAINT", format!("{path}.last_unix_ms")));
    }
    Ok(())
}

fn v_split(v: &Value, path: &str) -> Result<(), SpecError> {
    tag(v, path, &["last_day_holdout.v1", "none.v1"])?;
    walk(v, path, &["kind"], |_, _, _| Ok(()))
}

fn v_params_survivor(v: &Value, path: &str) -> Result<(), SpecError> {
    let hi = unlimited();
    walk(v, path, &SURVIVOR_KEYS, |key, val, p| match key {
        "max_age_min" => grid_int(val, p, 1, 1440).map(|_| ()),
        "min_age_min" => grid_int(val, p, 0, 1440).map(|_| ()),
        "min_buyers" => grid_int(val, p, 1, 100_000).map(|_| ()),
        "min_growth" => grid_decimal(val, p, 0, hi, false, true),
        "min_mcap" => grid_decimal(val, p, 0, hi, false, true),
        "min_ratio" => grid_decimal(val, p, 0, hi, false, true),
        _ => Ok(()),
    })?;
    let map = as_object(v, path)?;
    let min_age = grid_int(&map["min_age_min"], path, 0, 1440)?;
    let max_age = grid_int(&map["max_age_min"], path, 1, 1440)?;
    if min_age >= max_age {
        return Err(SpecError::new("E_CONSTRAINT", format!("{path}.max_age_min")));
    }
    Ok(())
}

fn v_params_sniper(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &SNIPER_KEYS, |key, val, p| match key {
        "max_dev_lamports" => grid_int(val, p, 0, I64_MAX).map(|_| ()),
        "min_dev_lamports" => grid_int(val, p, 0, I64_MAX).map(|_| ()),
        "panic_lamports" => grid_int(val, p, 0, I64_MAX).map(|_| ()),
        "stall_s" => grid_int(val, p, 1, 86_400).map(|_| ()),
        _ => Ok(()),
    })?;
    let map = as_object(v, path)?;
    let min_dev = grid_int(&map["min_dev_lamports"], path, 0, I64_MAX)?;
    let max_dev = grid_int(&map["max_dev_lamports"], path, 0, I64_MAX)?;
    if min_dev > max_dev {
        return Err(SpecError::new(
            "E_CONSTRAINT",
            format!("{path}.max_dev_lamports"),
        ));
    }
    Ok(())
}

fn v_params_graduacion(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &GRADUACION_KEYS, |key, val, p| match key {
        "abort_bps" => grid_int(val, p, 0, 10_000).map(|_| ()),
        "dip_bps" => grid_int(val, p, 0, 10_000).map(|_| ()),
        "timeout_min" => grid_int(val, p, 1, 1440).map(|_| ()),
        _ => Ok(()),
    })
}

fn v_strategy(v: &Value, path: &str) -> Result<(), SpecError> {
    let kind = tag(
        v,
        path,
        &["graduacion.v1", "sniper.v1", "survivor.v1"],
    )?;
    let params: fn(&Value, &str) -> Result<(), SpecError> = match kind {
        "survivor.v1" => v_params_survivor,
        "sniper.v1" => v_params_sniper,
        _ => v_params_graduacion,
    };
    walk(v, path, &["kind", "params"], |key, val, p| match key {
        "params" => params(val, p),
        _ => Ok(()),
    })
}

fn v_exit_policy(v: &Value, path: &str) -> Result<(), SpecError> {
    let hi = unlimited();
    walk(v, path, &EXIT_POLICY_KEYS, |key, val, p| match key {
        "hard_stop_bps" => grid_int(val, p, 1, 10_000).map(|_| ()),
        "time_stop_min" => grid_int(val, p, 1, 1440).map(|_| ()),
        "tp_mult" => grid_decimal(val, p, TEN18, hi, false, true),
        "tp_sell_bps" => grid_int(val, p, 1, 10_000).map(|_| ()),
        "trail_always" => v_bool(val, p),
        "trail_arm_bps" => grid_int(val, p, 0, 100_000).map(|_| ()),
        "trail_bps" => grid_int(val, p, 1, 10_000).map(|_| ()),
        "trail_sell_bps" => grid_int(val, p, 1, 10_000).map(|_| ()),
        _ => Ok(()),
    })
}

const TEN18: i128 = 1_000_000_000_000_000_000;

fn v_portfolio(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &PORTFOLIO_KEYS, |key, val, p| match key {
        "max_open_positions" => v_int(val, p, 1, 10_000).map(|_| ()),
        "notional_lamports" => v_int(val, p, 1, I64_MAX).map(|_| ()),
        _ => Ok(()),
    })
}

fn v_costs(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &COSTS_KEYS, |key, val, p| match key {
        "fee_lamports_per_tx" => v_int(val, p, 0, I64_MAX).map(|_| ()),
        "slippage_bps_per_side" => v_int(val, p, 0, 10_000).map(|_| ()),
        _ => Ok(()),
    })
}

fn v_inputs_sweep(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &INPUTS_KEYS, |key, val, p| match key {
        "costs" => v_costs(val, p),
        "dataset" => v_dataset(val, p),
        "exit_policy" => v_exit_policy(val, p),
        "portfolio" => v_portfolio(val, p),
        "split" => v_split(val, p),
        "strategy" => v_strategy(val, p),
        _ => Ok(()),
    })?;
    let map = as_object(v, path)?;
    let split_kind = match &map["split"] {
        Value::Object(s) => match s.get("kind") {
            Some(Value::Str(k)) => k.as_str(),
            _ => return Err(SpecError::new("E_TYPE", format!("{path}.split.kind"))),
        },
        _ => return Err(SpecError::new("E_TYPE", format!("{path}.split"))),
    };
    let days = match &map["dataset"] {
        Value::Object(d) => match d.get("days") {
            Some(Value::Int(n)) => *n,
            _ => return Err(SpecError::new("E_TYPE", format!("{path}.dataset.days"))),
        },
        _ => return Err(SpecError::new("E_TYPE", format!("{path}.dataset"))),
    };
    if split_kind == "last_day_holdout.v1" && days < 2 {
        return Err(SpecError::new("E_CONSTRAINT", format!("{path}.split")));
    }
    Ok(())
}

fn v_output_shape(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &OUTPUT_SHAPE_KEYS, |key, val, p| match key {
        "format" => v_enum(val, p, &["sweep_top.v1"]),
        "metrics" => {
            let items = v_array(val, p)?;
            for (i, item) in items.iter().enumerate() {
                v_enum(item, &format!("{p}[{i}]"), &METRICS)?;
            }
            for pair in items.windows(2) {
                let (a, b) = (v_str(&pair[0], p)?, v_str(&pair[1], p)?);
                if a.as_bytes() >= b.as_bytes() {
                    return Err(SpecError::new("E_NOT_SORTED", p));
                }
            }
            Ok(())
        }
        "rounding" => v_enum(val, p, &["trunc_to_lamports.v1"]),
        "top_n" => v_int(val, p, 1, 1000).map(|_| ()),
        _ => Ok(()),
    })
}

fn v_runner(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &RUNNER_KEYS, |key, val, p| match key {
        "commit" => v_shape(val, p, is_hex40),
        "entrypoint" => {
            let items = v_array(val, p)?;
            for (i, item) in items.iter().enumerate() {
                v_str(item, &format!("{p}[{i}]"))?;
            }
            Ok(())
        }
        "image_digest" => v_shape(val, p, is_sha256_ref),
        "image_ref" => v_str(val, p).map(|_| ()),
        "toolchain" => v_shape(val, p, is_toolchain),
        _ => Ok(()),
    })
}

fn v_limits(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &LIMITS_KEYS, |key, val, p| match key {
        "cpu_count" => v_int(val, p, 1, 64).map(|_| ()),
        "memory_bytes" => v_int(val, p, 1, 1i64 << 40).map(|_| ()),
        "wall_time_s" => v_int(val, p, 1, 86_400).map(|_| ()),
        _ => Ok(()),
    })
}

fn v_deadline(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &DEADLINE_KEYS, |key, val, p| match key {
        "accept_by_unix_s" => v_int(val, p, 1, I64_MAX).map(|_| ()),
        "deliver_within_s" => v_int(val, p, 1, 2_592_000).map(|_| ()),
        _ => Ok(()),
    })
}

fn v_payment(v: &Value, path: &str) -> Result<(), SpecError> {
    walk(v, path, &PAYMENT_KEYS, |key, val, p| match key {
        "amount_base_units" => v_int(val, p, 1, I64_MAX).map(|_| ()),
        "bond_base_units" => v_int(val, p, 0, I64_MAX).map(|_| ()),
        "mint" => v_shape(val, p, is_base58),
        "mint_decimals" => v_int(val, p, 0, 18).map(|_| ()),
        _ => Ok(()),
    })
}

fn v_proof_mode(v: &Value, path: &str) -> Result<(), SpecError> {
    let kind = tag(v, path, &["optimistic", "zk"])?;
    let declared: &[&str] = match kind {
        "optimistic" => &["challenge_window_s", "kind"],
        _ => &["kind", "verifier_key"],
    };
    walk(v, path, declared, |key, val, p| match key {
        "challenge_window_s" => v_int(val, p, 1, 2_592_000).map(|_| ()),
        "verifier_key" => v_shape(val, p, is_sha256_ref),
        _ => Ok(()),
    })
}

// ------------------------------------------------------------------------------ raíz

pub fn validate(order: &Value) -> Result<(), SpecError> {
    let map = match order {
        Value::Object(m) => m,
        _ => return Err(SpecError::new("E_NOT_OBJECT", "$")),
    };
    walk(order, "$", &ROOT_KEYS, |key, val, p| match key {
        "class" => v_enum(val, p, &["backtest.sweep.v1"]),
        "deadline" => v_deadline(val, p),
        "inputs" => {
            // `class` ordena antes que `inputs`, así que el recorrido canónico ya la validó
            // cuando se llega acá (SPEC §6.11, nota final).
            match map.get("class") {
                Some(Value::Str(s)) if s == "backtest.sweep.v1" => v_inputs_sweep(val, p),
                _ => Err(SpecError::new("E_ENUM", "$.class")),
            }
        }
        "limits" => v_limits(val, p),
        "output_shape" => v_output_shape(val, p),
        "payment" => v_payment(val, p),
        "proof_mode" => v_proof_mode(val, p),
        "runner" => v_runner(val, p),
        "schema_version" => match val {
            Value::Int(1) => Ok(()),
            Value::Int(_) => Err(SpecError::new("E_INT_RANGE", p)),
            _ => Err(SpecError::new("E_TYPE", p)),
        },
        _ => Ok(()),
    })
}
