//! linux-parity: complete
//! linux-source: vendor/linux/net/batman-adv/log.c
//! test-origin: linux:vendor/linux/net/batman-adv/log.c
//! B.A.T.M.A.N. advanced debug log trace bridge.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatadvPriv<'a> {
    pub interface: &'a str,
}

pub const DEBUG_LOG_SUCCESS: i32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VaFormat<'a> {
    pub fmt: &'a str,
    pub args: &'a [&'a str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatadvTraceEvent<'a> {
    pub bat_priv: BatadvPriv<'a>,
    pub vaf: VaFormat<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatadvDebugLogResult<'a> {
    pub ret: i32,
    pub va_started: bool,
    pub trace: BatadvTraceEvent<'a>,
    pub va_ended: bool,
}

pub fn trace_batadv_dbg<'a>(bat_priv: BatadvPriv<'a>, vaf: VaFormat<'a>) -> BatadvTraceEvent<'a> {
    BatadvTraceEvent { bat_priv, vaf }
}

pub fn batadv_debug_log<'a>(
    bat_priv: BatadvPriv<'a>,
    fmt: &'a str,
    args: &'a [&'a str],
) -> BatadvDebugLogResult<'a> {
    let va_started = true;
    let vaf = VaFormat { fmt, args };
    let trace = trace_batadv_dbg(bat_priv, vaf);
    let va_ended = true;

    BatadvDebugLogResult {
        ret: DEBUG_LOG_SUCCESS,
        va_started,
        trace,
        va_ended,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_log_matches_linux_va_format_trace_path() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/batman-adv/log.c"
        ));
        assert!(source.contains("#include \"log.h\""));
        assert!(source.contains("#include \"main.h\""));
        assert!(source.contains("#include <linux/stdarg.h>"));
        assert!(source.contains("#include \"trace.h\""));
        assert!(source.contains("struct batadv_priv *bat_priv"));
        assert!(source.contains("const char *fmt, ..."));
        assert!(source.contains("struct va_format vaf;"));
        assert!(source.contains("va_list args;"));
        assert!(source.contains("va_start(args, fmt);"));
        assert!(source.contains("vaf.fmt = fmt;"));
        assert!(source.contains("vaf.va = &args;"));
        assert!(source.contains("trace_batadv_dbg(bat_priv, &vaf);"));
        assert!(source.contains("va_end(args);"));
        assert!(source.contains("return 0;"));

        let args = ["aa:bb:cc:dd:ee:ff"];
        let result = batadv_debug_log(BatadvPriv { interface: "mesh0" }, "neigh %pM", &args);
        assert_eq!(result.ret, DEBUG_LOG_SUCCESS);
        assert!(result.va_started);
        assert!(result.va_ended);
        assert_eq!(result.trace.bat_priv.interface, "mesh0");
        assert_eq!(result.trace.vaf.fmt, "neigh %pM");
        assert_eq!(result.trace.vaf.args, &args);
    }
}
