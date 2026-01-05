//! Shell completion generation.
//!
//! Generates completion scripts for bash, zsh, and fish shells.

use crate::errors::Result;
use clap::Parser;

/// Arguments for shell completion generation.
#[derive(Parser)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_parser = clap::builder::PossibleValuesParser::new(["bash", "zsh", "fish"]))]
    pub shell: String,
}

/// Generate shell completion script.
pub fn execute(args: CompletionsArgs) -> Result<()> {
    match args.shell.as_str() {
        "bash" => print_bash_completion(),
        "zsh" => print_zsh_completion(),
        "fish" => print_fish_completion(),
        _ => unreachable!(),
    }
    Ok(())
}

/// Print bash completion script.
fn print_bash_completion() {
    println!(
        r#"_standby_completions() {{
    local cur prev words cword
    COMPREPLY=()
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    # Main subcommands
    local subcommands="sleep timeout wait"

    case "$prev" in
        standby)
            COMPREPLY=($(compgen -W "$subcommands" -- "$cur"))
            return 0
            ;;
        timeout)
            COMPREPLY=($(compgen -W "-s --signal -k --kill-after --preserve-status --foreground -v --verbose" -- "$cur"))
            return 0
            ;;
        -s|--signal)
            COMPREPLY=($(compgen -W "TERM KILL INT STOP CONT TSTP HUP 15 9 2 19 18 20 1" -- "$cur"))
            return 0
            ;;
    esac

    # Default to files
    if [[ "$cur" == -* ]]; then
        case "${{COMP_WORDS[1]}}" in
            timeout)
                COMPREPLY=($(compgen -W "-s --signal -k --kill-after --preserve-status --foreground -v --verbose" -- "$cur"))
                ;;
            sleep)
                COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
                ;;
            wait)
                COMPREPLY=($(compgen -W "--timeout -h --help" -- "$cur"))
                ;;
        esac
    else
        COMPREPLY=($(compgen -f -- "$cur"))
    fi
}}

complete -o bashdefault -o default -o nospace -F _standby_completions standby
"#
    );
}

/// Print zsh completion script.
fn print_zsh_completion() {
    println!(
        r#"#compdef standby

_standby() {{
    local curcontext="$curcontext" state line expl
    typeset -A opt_args

    _arguments \
        '(- *)::command:(sleep timeout wait)' \
        '*::arg:->args'

    case $state in
        args)
            case $line[1] in
                timeout)
                    _arguments \
                        '-s+[signal]:signal:(TERM KILL INT STOP CONT TSTP HUP 15 9 2 19 18 20 1)' \
                        '-k+[kill-after]:duration:' \
                        '--preserve-status[preserve exit status]' \
                        '--foreground[run in foreground]' \
                        '-v[verbose output]'
                    ;;
                sleep)
                    _message 'duration'
                    ;;
                wait)
                    _arguments \
                        '--timeout+[timeout duration]:duration:'
                    ;;
            esac
            ;;
    esac
}}

_standby
"#
    );
}

/// Print fish completion script.
fn print_fish_completion() {
    println!(
        r#"# Fish shell completions for standby

complete -c standby -f -n "__fish_use_subcommand_from_list sleep timeout wait"
complete -c standby -f -n "not __fish_seen_subcommand_from sleep timeout wait" -a "sleep" -d "Suspend execution"
complete -c standby -f -n "not __fish_seen_subcommand_from sleep timeout wait" -a "timeout" -d "Run with time limit"
complete -c standby -f -n "not __fish_seen_subcommand_from sleep timeout wait" -a "wait" -d "Wait for process"

# Timeout completions
complete -c standby -f -n "__fish_seen_subcommand_from timeout" -s s -l signal -d "Signal to send" -a "TERM KILL INT STOP CONT TSTP HUP"
complete -c standby -f -n "__fish_seen_subcommand_from timeout" -s k -l kill-after -d "Kill after duration"
complete -c standby -f -n "__fish_seen_subcommand_from timeout" -l preserve-status -d "Preserve exit status"
complete -c standby -f -n "__fish_seen_subcommand_from timeout" -l foreground -d "Run in foreground"
complete -c standby -f -n "__fish_seen_subcommand_from timeout" -s v -l verbose -d "Verbose output"

# Wait completions
complete -c standby -f -n "__fish_seen_subcommand_from wait" -l timeout -d "Timeout duration"
"#
    );
}
