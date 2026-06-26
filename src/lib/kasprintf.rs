//! linux-parity: complete
//! linux-source: vendor/linux/lib/kasprintf.c
//! test-origin: linux:vendor/linux/lib/kasprintf.c
//! Kernel asprintf helpers with `%s` const-string fast paths.

extern crate alloc;

use alloc::string::String;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KvasprintfConst<'a> {
    Const(&'a str),
    Allocated(String),
}

pub fn kvasprintf(fmt: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut arg_index = 0usize;
    let mut chars = fmt.chars();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('%') => out.push('%'),
            Some('s') => {
                if let Some(arg) = args.get(arg_index) {
                    out.push_str(arg);
                    arg_index += 1;
                }
            }
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }

    out
}

pub fn kvasprintf_const<'a>(fmt: &'a str, args: &'a [&'a str]) -> KvasprintfConst<'a> {
    if !fmt.as_bytes().contains(&b'%') {
        return KvasprintfConst::Const(fmt);
    }
    if fmt == "%s" {
        return KvasprintfConst::Const(args.first().copied().unwrap_or(""));
    }
    KvasprintfConst::Allocated(kvasprintf(fmt, args))
}

pub fn kasprintf(fmt: &str, args: &[&str]) -> String {
    kvasprintf(fmt, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kasprintf_helpers_match_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/kasprintf.c"
        ));
        assert!(source.contains("first = vsnprintf(NULL, 0, fmt, aq);"));
        assert!(source.contains("p = kmalloc_track_caller(first+1, gfp);"));
        assert!(source.contains("second = vsnprintf(p, first+1, fmt, ap);"));
        assert!(source.contains("WARN(first != second"));
        assert!(source.contains("if (!strchr(fmt, '%'))"));
        assert!(source.contains("if (!strcmp(fmt, \"%s\"))"));
        assert!(source.contains("return kstrdup_const(va_arg(ap, const char*), gfp);"));
        assert!(source.contains("EXPORT_SYMBOL(kvasprintf);"));
        assert!(source.contains("EXPORT_SYMBOL(kvasprintf_const);"));
        assert!(source.contains("EXPORT_SYMBOL(kasprintf);"));

        assert_eq!(kvasprintf("net/%s/%s", &["eth0", "rx"]), "net/eth0/rx");
        assert_eq!(kvasprintf("100%% %s", &["ok"]), "100% ok");
        assert_eq!(
            kvasprintf_const("literal", &[]),
            KvasprintfConst::Const("literal")
        );
        assert_eq!(
            kvasprintf_const("%s", &["already-const"]),
            KvasprintfConst::Const("already-const")
        );
        assert_eq!(
            kvasprintf_const("%s/%s", &["a", "b"]),
            KvasprintfConst::Allocated(String::from("a/b"))
        );
        assert_eq!(kasprintf("%s", &["value"]), "value");
    }
}
