use pgrx::{prelude::*, JsonB};
use std::fmt;
use supabase_wrappers::prelude::report_warning;

// fdw stats table name
const FDW_STATS_TABLE: &str = "wrappers_fdw_stats";

// metric list
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Metric {
    CreateTimes,
    RowsIn,
    RowsOut,
    BytesIn,
    BytesOut,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Metric::CreateTimes => write!(f, "create_times"),
            Metric::RowsIn => write!(f, "rows_in"),
            Metric::RowsOut => write!(f, "rows_out"),
            Metric::BytesIn => write!(f, "bytes_in"),
            Metric::BytesOut => write!(f, "bytes_out"),
        }
    }
}

// get stats table full qualified name
fn get_stats_table() -> String {
    let sql = format!(
        "select b.nspname || '.{}'
         from pg_catalog.pg_extension a join pg_namespace b on a.extnamespace = b.oid
         where a.extname = 'wrappers'",
        FDW_STATS_TABLE
    );
    Spi::get_one(&sql)
        .expect("wrappers extension should be installed")
        .expect("fdw stats table should be created")
}

fn is_txn_read_only() -> bool {
    Spi::get_one("show transaction_read_only") == Ok(Some("on"))
}

// increase stats value
#[allow(dead_code)]
pub(crate) fn inc_stats(fdw_name: &str, metric: Metric, inc: i64) {
    if is_txn_read_only() {
        return;
    }

    let sql = format!(
        "insert into {} as s (fdw_name, {}) values($1, $2)
         on conflict(fdw_name)
         do update set
            {} = coalesce(s.{}, 0) + excluded.{},
            updated_at = timezone('utc'::text, now())",
        get_stats_table(),
        metric,
        metric,
        metric,
        metric
    );
    Spi::run_with_args(
        &sql,
        &[fdw_name.into(), inc.into()],
    )
    .expect("should insert into fdw stats table");
}

// get metadata
#[allow(dead_code)]
pub(crate) fn get_metadata(fdw_name: &str) -> Option<JsonB> {
    let sql = format!(
        "select metadata from {} where fdw_name = $1",
        get_stats_table()
    );
    Spi::get_one_with_args(
        &sql,&[fdw_name.into()]
    )
    .unwrap_or_default()
}

// set metadata
#[allow(dead_code)]
pub(crate) fn set_metadata(fdw_name: &str, metadata: Option<JsonB>) {
    if is_txn_read_only() {
        return;
    }

    let sql = format!(
        "insert into {} as s (fdw_name, metadata) values($1, $2)
         on conflict(fdw_name)
         do update set
            metadata = $2,
            updated_at = timezone('utc'::text, now())",
        get_stats_table()
    );
    if let Err(err) = Spi::run_with_args(
        &sql,
        &[fdw_name.into(), metadata.into()]
    ) {
        report_warning(&format!("set fdw stats metadata failed: {}", err));
    };
}
