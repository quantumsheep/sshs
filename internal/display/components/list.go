package components

import (
	"strings"

	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/samber/lo"
)

var (
	listStyle = lipgloss.NewStyle()
)

type ListItem struct {
	ID any

	Name    string
	Details string
}

func (i *ListItem) Title() string       { return i.Name }
func (i *ListItem) Description() string { return i.Details }
func (i *ListItem) FilterValue() string { return i.Name }

type ListComponent struct {
	Model list.Model
	Style lipgloss.Style
}

type OnSelectFunc func(*ListItem)

type ListComponentConfig struct {
	Items               []*ListItem
	DefaultSearchFilter string
	OnSelect            OnSelectFunc
}

func NewListComponent(config *ListComponentConfig) *ListComponent {
	listModelItems := lo.Map(config.Items, func(item *ListItem, _ int) list.Item {
		return list.Item(item)
	})

	maxHeight := 2
	for _, item := range config.Items {
		if item.Details == "" {
			continue
		}

		height := strings.Count(item.Details, "\n") + 2
		if height > maxHeight {
			maxHeight = height
		}
	}

	delegate := newListDelegate(config.OnSelect)
	delegate.SetHeight(maxHeight)

	listModel := list.New(listModelItems, delegate, 0, 0)
	listModel.Title = "SSHS"
	listModel.SetShowStatusBar(false)

	return &ListComponent{
		Model: listModel,
		Style: listStyle,
	}
}

func (c *ListComponent) Init() tea.Cmd {
	return tea.EnterAltScreen
}

func (c *ListComponent) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		h, v := c.Style.GetFrameSize()
		c.Model.SetSize(msg.Width-h, msg.Height-v)
	}

	var cmd tea.Cmd
	c.Model, cmd = c.Model.Update(msg)
	return c, cmd
}

func (c *ListComponent) View() string {
	return c.Style.Render(c.Model.View())
}

func newListDelegate(onSelect OnSelectFunc) list.DefaultDelegate {
	d := list.NewDefaultDelegate()

	d.UpdateFunc = func(msg tea.Msg, m *list.Model) tea.Cmd {
		switch msg := msg.(type) {
		case tea.KeyMsg:
			switch msg.Type {
			case tea.KeyEnter:
				onSelect(m.SelectedItem().(*ListItem))
			}
		}

		return nil
	}

	return d
}
