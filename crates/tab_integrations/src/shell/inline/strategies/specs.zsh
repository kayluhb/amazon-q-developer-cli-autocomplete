
#--------------------------------------------------------------------#
# Specs Suggestion Strategy                                          #
#--------------------------------------------------------------------#
# Suggests the autocomplete popup's top suggestion, which tabterm
# caches for the current buffer.
#

_tab_autosuggest_strategy_specs() {
	typeset -g suggestion="$(command -v {{CLI_BINARY_NAME}} >/dev/null 2>&1 && {{CLI_BINARY_NAME}} _ inline-suggestion --buffer "${BUFFER}" 2>/dev/null)"
}
