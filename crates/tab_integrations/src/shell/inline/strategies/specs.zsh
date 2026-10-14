
#--------------------------------------------------------------------#
# Specs Suggestion Strategy                                          #
#--------------------------------------------------------------------#
# Suggests the autocomplete popup's top suggestion, which tabterm
# caches for the current buffer. The buffer goes over stdin, not
# argv, so other users can't read it with ps.
#

_tab_autosuggest_strategy_specs() {
	[[ -n "${TABTERM_SESSION_ID}" ]] || return
	typeset -g suggestion="$(command -v {{CLI_BINARY_NAME}} >/dev/null 2>&1 && print -rn -- "$1" | {{CLI_BINARY_NAME}} _ inline-suggestion 2>/dev/null)"
}
