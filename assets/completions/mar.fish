complete -c mar -l color -d 'When to style printed output: `auto` styles only when writing to a terminal' -r -f -a "auto\t''
always\t''
never\t''"
complete -c mar -l width -d 'Maximum content width in columns' -r
complete -c mar -l theme -d 'Color preset: dark, light, ansi, or a theme file name' -r
complete -c mar -l config -d 'Config file to use instead of $XDG_CONFIG_HOME/marustdown/config.toml' -r -F
complete -c mar -l cat -d 'Print the rendered document and exit instead of paging (automatic when stdout isn\'t a terminal)'
complete -c mar -l no-color -d 'Same as `--color never` (NO_COLOR is honored too)'
complete -c mar -l watch -d 'Reload the file when it changes on disk (also `watch = true` in the config)'
complete -c mar -l no-watch -d 'Don\'t watch the file, even if the config enables it'
complete -c mar -l no-icons -d 'Use ASCII glyphs instead of icons'
complete -c mar -s h -l help -d 'Print help (see more with \'--help\')'
complete -c mar -s V -l version -d 'Print version'
