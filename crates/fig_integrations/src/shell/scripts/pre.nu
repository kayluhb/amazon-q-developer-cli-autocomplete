mkdir ~/.local/bin | ignore

def pathadd [path: string] {
  if not ($env.PATH | any {|it| $it == $path }) {
    $env.PATH | prepend $path
  } else {
    $env.PATH
  }
}

let-env PATH = pathadd $"($env.HOME)/.local/bin"
let-env PATH = pathadd $"($env.HOME)/.local/bin"

if "TAB_NEW_SESSION" in $env {
  let-env TABTERM_SESSION_ID = $nothing
  let-env TAB_TERM = $nothing
  let-env TAB_NEW_SESSION = $nothing
}

if "TAB_SET_PARENT_CHECK" not-in $env {
  if "TAB_PARENT" not-in $env and "TAB_SET_PARENT" in $env {
    let-env TAB_PARENT = $env.TAB_SET_PARENT
    let-env TAB_SET_PARENT = $nothing
  }
  let-env TAB_SET_PARENT_CHECK = 1
}


let result = (^{{CLI_BINARY_NAME}} _ should-figterm-launch | complete)
let-env SHOULD_TABTERM_LAUNCH = $result.exit_code

let should_launch = (
    ("PROCESS_LAUNCHED_BY_TAB" not-in $env or ($env.PROCESS_LAUNCHED_BY_TAB | str length) == 0)
    and ($env.SHOULD_TABTERM_LAUNCH == 0 or
       ($env.SHOULD_TABTERM_LAUNCH == 2 and "TAB_TERM" not-in $env))
)

if $should_launch {
  let TAB_SHELL = ({{CLI_BINARY_NAME}} _ get-shell | complete).stdout
  
  let fig_term_name = "nu (figterm)"
  let figterm_path = if ([$env.HOME ".fig" "bin" $fig_term_name] | path join | path exists) {
    [$env.HOME ".fig" "bin" $fig_term_name] | path join
  } else if (which figterm | length) > 0 {
    which figterm | first | get path
  } else {
    [$env.HOME ".fig" "bin" "figterm"] | path join
  }

  with-env {
    TAB_SHELL: $TAB_SHELL
  } {
    exec $figterm_path
  }
}
