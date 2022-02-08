package ui

import (
	"github.com/gdamore/tcell/v2"
	"github.com/rivo/tview"
)

type MultiFocusFlexItem struct {
	tview.Primitive
	Focus bool
}

type MultiFocusFlex struct {
	*tview.Flex
	items []*MultiFocusFlexItem
}

func NewMultiFlex() *MultiFocusFlex {
	return &MultiFocusFlex{
		Flex:  tview.NewFlex(),
		items: make([]*MultiFocusFlexItem, 0),
	}
}

func (f *MultiFocusFlex) InputHandler() func(event *tcell.EventKey, setFocus func(p tview.Primitive)) {
	return f.WrapInputHandler(func(event *tcell.EventKey, setFocus func(p tview.Primitive)) {
		for _, item := range f.items {
			if item != nil && item.Focus {
				if handler := item.InputHandler(); handler != nil {
					handler(event, setFocus)
				}
			}
		}
	})
}

func (f *MultiFocusFlex) AddItem(item tview.Primitive, fixedSize, proportion int, focus bool) *MultiFocusFlex {
	f.items = append(f.items, &MultiFocusFlexItem{
		Primitive: item,
		Focus:     focus,
	})

	f.Flex.AddItem(item, fixedSize, proportion, focus)
	return f
}

func (f *MultiFocusFlex) RemoveItem(p tview.Primitive) *MultiFocusFlex {
	for index := len(f.items) - 1; index >= 0; index-- {
		if f.items[index].Primitive == p {
			f.items = append(f.items[:index], f.items[index+1:]...)
		}
	}

	f.Flex.RemoveItem(p)
	return f
}
