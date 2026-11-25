//! db_query / db_get / db_run frente a better-sqlite3 12.10 (valores capturados en Electron 41).
//! Nota: en el cable JSON un REAL entero sale como `5.0`; JS lo lee como 5. Por eso se compara numéricamente.
use cuentacobro_lib::{db, generic::*};
use serde_json::{Value, json};

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y)
                    .all(|((k1, v1), (k2, v2))| k1 == k2 && same(v1, v2))
        }
        _ => a == b,
    }
}
macro_rules! ok {
    ($r:expr, $e:expr) => {{
        let got = json!($r.unwrap());
        assert!(same(&got, &$e), "got {got} expected {}", $e);
    }};
}
macro_rules! err {
    ($r:expr, $m:expr) => {{
        assert_eq!($r.unwrap_err(), $m);
    }};
}
fn p(v: Value) -> Option<Vec<Value>> {
    v.as_array().cloned()
}

#[test]
fn generic_sql_like_better_sqlite3() {
    let dir = std::env::temp_dir().join(format!("generic-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let c = db::open_connection(&dir.join("g.sqlite")).unwrap();
    ok!(
        db_get(&c, "select typeof(?) t, ? v", p(json!([5, 5]))),
        json!({"t": "real", "v": 5})
    );
    ok!(
        db_get(&c, "select typeof(?) t", p(json!([0.5]))),
        json!({"t": "real"})
    );
    err!(
        db_get(&c, "select typeof(?) t", p(json!([true]))),
        ERR_BIND_TYPE
    );
    err!(db_get(&c, "select ?", p(json!([[1]]))), ERR_BIND_TYPE);
    err!(db_get(&c, "select ?", p(json!([{}]))), ERR_BIND_TYPE);
    ok!(
        db_get(&c, "select typeof(?) t", p(json!([null]))),
        json!({"t": "null"})
    );
    ok!(
        db_get(
            &c,
            "select typeof(?) t, ? v",
            p(json!([9007199254740994u64, 9007199254740994u64]))
        ),
        json!({"t": "real", "v": 9007199254740994u64})
    );
    ok!(
        db_get(&c, "select typeof(?) t, ? v", p(json!([-0.0, -0.0]))),
        json!({"t": "real", "v": 0})
    );
    assert_eq!(db_get(&c, "select 1 where 0", p(json!([]))).unwrap(), None); // JS: undefined
    ok!(db_query(&c, "select 1 where 0", None), json!([]));
    ok!(
        db_run(&c, "select 1", None),
        json!({"changes": 0, "lastInsertRowid": 0})
    );
    err!(db_query(&c, "create table x(a)", None), ERR_NOT_READER);
    err!(
        db_get(&c, "create table if not exists x(a)", None),
        ERR_NOT_READER
    );
    ok!(
        db_query(&c, "select name from sqlite_master where name = 'x'", None),
        json!([])
    ); // no se ejecutó
    ok!(
        db_run(
            &c,
            "create table y(a TEXT, b DECIMAL(10,2), c INTEGER, d)",
            None
        ),
        json!({"changes": 0, "lastInsertRowid": 0})
    );
    ok!(
        db_run(
            &c,
            "insert into y values (?,?,?,?)",
            p(json!([1, 1, 1.0, 1]))
        ),
        json!({"changes": 1, "lastInsertRowid": 1})
    );
    ok!(
        db_run(
            &c,
            "insert into y values (?,?,?,?)",
            p(json!([1.5, 1.5, 1.5, 1.5]))
        ),
        json!({"changes": 1, "lastInsertRowid": 2})
    );
    ok!(
        db_run(
            &c,
            "insert into y values (?,?,?,?)",
            p(json!(["1.50", "1.50", "2", "1.50"]))
        ),
        json!({"changes": 1, "lastInsertRowid": 3})
    );
    ok!(
        db_run(
            &c,
            "insert into y values (?,?,?,?)",
            p(json!(["abc", "abc", "abc", "abc"]))
        ),
        json!({"changes": 1, "lastInsertRowid": 4})
    );
    ok!(
        db_query(
            &c,
            "select a, typeof(a) ta, b, typeof(b) tb, c, typeof(c) tc, d, typeof(d) td from y",
            None
        ),
        json!([
        {"a":"1.0","ta":"text","b":1,"tb":"integer","c":1,"tc":"integer","d":1,"td":"real"},
        {"a":"1.5","ta":"text","b":1.5,"tb":"real","c":1.5,"tc":"real","d":1.5,"td":"real"},
        {"a":"1.50","ta":"text","b":1.5,"tb":"real","c":2,"tc":"integer","d":"1.50","td":"text"},
        {"a":"abc","ta":"text","b":"abc","tb":"text","c":"abc","tc":"text","d":"abc","td":"text"}])
    );
    err!(db_get(&c, "select ?, ?", p(json!([1]))), ERR_TOO_FEW);
    err!(db_get(&c, "select ?", p(json!([1, 2]))), ERR_TOO_MANY);
    err!(db_get(&c, "select @a", p(json!([]))), ERR_MISSING_NAMED);
    err!(db_get(&c, "select @a", p(json!([1]))), ERR_TOO_MANY);
    err!(db_get(&c, "select ?, @a", p(json!([1]))), ERR_MISSING_NAMED);
    err!(db_get(&c, "select ?1 v", p(json!([5]))), ERR_TOO_MANY);
    err!(db_get(&c, "select :x", p(json!([]))), ERR_MISSING_NAMED);
    err!(db_get(&c, "select $x", p(json!([]))), ERR_MISSING_NAMED);
    err!(db_get(&c, "select 1; select 2", None), ERR_MULTIPLE);
    for empty in ["", "   ", ";", "-- hi"] {
        err!(db_get(&c, empty, None), ERR_NO_STATEMENTS);
    }
    ok!(db_get(&c, "select 1 v;;", None), json!({"v": 1}));
    ok!(db_get(&c, "select 1 v; /* c */ ", None), json!({"v": 1}));
    err!(db_get(&c, "selec 1", None), "near \"selec\": syntax error");
    err!(
        db_query(&c, "select * from nope", None),
        "no such table: nope"
    );
    err!(
        db_get(&c, "select \"abc\" v", None),
        "no such column: \"abc\" - should this be a string literal in single-quotes?"
    );
    ok!(
        db_get(
            &c,
            "select 9007199254740993 v, typeof(9007199254740993) t",
            None
        ),
        json!({"v": 9007199254740993i64, "t": "integer"})
    ); // JS lee 9007199254740992
    ok!(db_get(&c, "select x'0102' v", None), json!({"v": [1, 2]}));
    ok!(db_get(&c, "select 1 a, 2 a", None), json!({"a": 2}));
    ok!(
        db_get(&c, "select 1+1, count(*) from y", None),
        json!({"1+1": 2, "count(*)": 4})
    );
    ok!(
        db_run(&c, "update y set a = a", None),
        json!({"changes": 4, "lastInsertRowid": 4})
    );
    ok!(
        db_run(&c, "delete from y where c = 2", None),
        json!({"changes": 1, "lastInsertRowid": 4})
    );
    ok!(
        db_get(&c, "select 2.0 v, 1e21 w, 0.1+0.2 x, -0.0 z", None),
        json!({"v": 2, "w": 1e21, "x": 0.30000000000000004, "z": 0})
    );
    ok!(
        db_get(&c, "select 9e999 v, -9e999 w", None),
        json!({"v": null, "w": null})
    ); // Electron entregaba ±Infinity; JSON no puede
    ok!(
        db_run(&c, "BEGIN", None),
        json!({"changes": 0, "lastInsertRowid": 4})
    );
    ok!(
        db_run(&c, "COMMIT", None),
        json!({"changes": 0, "lastInsertRowid": 4})
    );
    ok!(
        db_query(&c, "PRAGMA foreign_keys", None),
        json!([{"foreign_keys": 1}])
    );
    ok!(
        db_run(&c, "PRAGMA foreign_keys = OFF", None),
        json!({"changes": 0, "lastInsertRowid": 4})
    );
    ok!(
        db_query(&c, "PRAGMA foreign_keys", None),
        json!([{"foreign_keys": 0}])
    );
    db_run(&c, "PRAGMA foreign_keys = ON", None).unwrap();
    db_run(&c, "create table p(a TEXT)", None).unwrap();
    db_run(&c, "insert into p values (?)", p(json!([3001234567u64]))).unwrap();
    ok!(
        db_get(&c, "select a, typeof(a) t from p", None),
        json!({"a": "3001234567.0", "t": "text"})
    );
    db_run(&c, "create table r(a)", None).unwrap();
    ok!(
        db_query(&c, "insert into r (a) values ('r') returning rowid", None),
        json!([{"rowid": 1}])
    );
    ok!(
        db_run(&c, "insert into r (a) values ('s') returning rowid", None),
        json!({"changes": 1, "lastInsertRowid": 2})
    );
    db_run(&c, "create table pa(id integer primary key)", None).unwrap();
    db_run(&c, "create table ch(pid references pa(id))", None).unwrap();
    err!(
        db_run(&c, "insert into ch values (?)", p(json!([7]))),
        "FOREIGN KEY constraint failed"
    );
    db_run(
        &c,
        "create table pr(id integer primary key check (id = 1))",
        None,
    )
    .unwrap();
    err!(
        db_run(&c, "insert into pr (id) values (?)", p(json!([2]))),
        "CHECK constraint failed: id = 1"
    );
    db_run(&c, "create table nn(a not null)", None).unwrap();
    err!(
        db_run(&c, "insert into nn values (?)", p(json!([null]))),
        "NOT NULL constraint failed: nn.a"
    );
    db_run(&c, "create table uq(a unique)", None).unwrap();
    db_run(&c, "insert into uq values (1)", None).unwrap();
    err!(
        db_run(&c, "insert into uq values (1)", None),
        "UNIQUE constraint failed: uq.a"
    );
    drop(c);
    std::fs::remove_dir_all(&dir).ok();
}

/// Una BD en WAL (p. ej. restaurada desde un archivo editado con otra herramienta) vuelve a
/// DELETE al abrirla y consolida los registros que estaban en el `-wal`.
#[test]
fn open_connection_sale_de_wal_sin_perder_datos() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live.sqlite");
    let writer = rusqlite::Connection::open(&live).unwrap();
    writer.execute_batch("CREATE TABLE t (v INTEGER);").unwrap();
    let mode: String = writer
        .pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    writer
        .execute_batch("PRAGMA wal_autocheckpoint = 0; INSERT INTO t VALUES (1), (2);")
        .unwrap();
    // Copia "en caliente": las dos filas solo existen en el -wal.
    let copy = dir.path().join("copy.sqlite");
    std::fs::copy(&live, &copy).unwrap();
    std::fs::copy(
        dir.path().join("live.sqlite-wal"),
        dir.path().join("copy.sqlite-wal"),
    )
    .unwrap();
    drop(writer);

    let conn = db::open_connection(&copy).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "delete");
    drop(conn);
    assert!(!dir.path().join("copy.sqlite-wal").exists());

    // Verifica que SQLite consolidó los registros en el archivo principal.
    let backup = dir.path().join("backup.sqlite");
    std::fs::copy(&copy, &backup).unwrap();
    let b = rusqlite::Connection::open(&backup).unwrap();
    let n: i64 = b
        .query_row("SELECT count(*) FROM t", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
}
