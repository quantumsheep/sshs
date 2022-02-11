package ui

import (
	"github.com/gdamore/tcell/v2"
	"github.com/rivo/tview"
)

type SearchBar struct {
	*tview.InputField
}

func NewSearchBar(filter string) *SearchBar {
	searchBar := &SearchBar{
		InputField: tview.NewInputField(),
	}

	searchBar.
		SetText(filter).
		SetPlaceholder("Search...").
		SetPlaceholderStyle(tcell.StyleDefault).
		SetPlaceholderTextColor(tcell.ColorYellow).
		SetFieldBackgroundColor(tcell.ColorReset)

	searchBar.
		SetBorder(true).
		SetBorderPadding(0, 0, 1, 0).
		SetTitleAlign(tview.AlignLeft).
		SetBackgroundColor(tcell.ColorReset)

	return searchBar
}
