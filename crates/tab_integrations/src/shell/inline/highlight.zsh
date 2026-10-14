
#--------------------------------------------------------------------#
# Highlighting                                                       #
#--------------------------------------------------------------------#

# If there was a highlight, remove it
_tab_autosuggest_highlight_reset() {
	typeset -g _TAB_AUTOSUGGEST_LAST_HIGHLIGHT

	if [[ -n "$_TAB_AUTOSUGGEST_LAST_HIGHLIGHT" ]]; then
		region_highlight=("${(@)region_highlight:#$_TAB_AUTOSUGGEST_LAST_HIGHLIGHT}")
		unset _TAB_AUTOSUGGEST_LAST_HIGHLIGHT
	fi
}

# If there's a suggestion, highlight it
_tab_autosuggest_highlight_apply() {
	typeset -g _TAB_AUTOSUGGEST_LAST_HIGHLIGHT

	if (( $#POSTDISPLAY )); then
		typeset -g _TAB_AUTOSUGGEST_LAST_HIGHLIGHT="$#BUFFER $(($#BUFFER + $#POSTDISPLAY)) $TAB_AUTOSUGGEST_HIGHLIGHT_STYLE"
		region_highlight+=("$_TAB_AUTOSUGGEST_LAST_HIGHLIGHT")
	else
		unset _TAB_AUTOSUGGEST_LAST_HIGHLIGHT
	fi
}
