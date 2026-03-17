    pub const CMD_SHIMS_CONTENT: &str = r##"@goto #_undefined_# 2>NUL || @title %COMSPEC% & @setlocal & node "%~dp0\..\bin\wrangler.js" %*"##;
pub const BASH_SHIMS_CONTENT: &str = r##"
#!/usr/bin/env bash
basedir=$(dirname "$0")

case "$(uname -s)" in
  *CYGWIN*) basedir=$(cygpath -w "$basedir");;
  *MSYS*) basedir=$(cygpath -w "$basedir");;
esac

exec node "$basedir/../bin/wrangler.js" "$@"

ret=$?
exit $ret
"##;
