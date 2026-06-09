// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §8 Group and layout
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct GroupDecl {
    pub name: String,
    pub items: Vec<GroupItem>,
}

#[derive(Debug, Clone)]
pub enum GroupItem {
    Contains(Vec<String>),
    Arrange(ArrangeMode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrangeMode {
    Grid,
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowDirection {
    LeftToRight,
    TopToBottom,
    RightToLeft,
    BottomToTop,
}

