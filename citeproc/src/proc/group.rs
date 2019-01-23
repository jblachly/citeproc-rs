// cs:group implicitly acts as a conditional: cs:group and its child elements are suppressed if a)
// at least one rendering element in cs:group calls a variable (either directly or via a macro),
// and b) all variables that are called are empty. This accommodates descriptive cs:text elements.
//
// Make a new one of these per <group> subtree.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GroupVars {
    /// A group has only seen stuff like `<text value=""/>` so far
    NoneSeen,
    /// Renderer encountered >= 1 variables, but did not render any of them
    OnlyEmpty,
    /// Renderer encountered >= 1 variables that it did render
    DidRender,
}

use self::GroupVars::*;

impl GroupVars {
    #[inline]
    pub fn new() -> Self {
        NoneSeen
    }

    #[inline]
    pub fn rendered_if(b: bool) -> Self {
        if b {
            GroupVars::DidRender
        } else {
            GroupVars::OnlyEmpty
        }
    }

    #[inline]
    pub fn did_not_render(self) -> Self {
        match self {
            DidRender => DidRender,
            _ => OnlyEmpty
        }
    }

    #[inline]
    pub fn did_render(self) -> Self {
        DidRender
    }

    pub fn with_subtree(self, subtree: Self) -> Self {
        match subtree {
            NoneSeen => self,
            OnlyEmpty => self.did_not_render(),
            DidRender => DidRender,
        }
    }

    /// Say you have
    ///
    /// ```xml
    /// <group>
    ///   <text value="tag" />
    ///   <text variable="var" />
    /// </group>
    /// ```
    ///
    /// The tag is `NoneSeen`, the var has `DidRender`.
    ///
    /// ```
    /// assert_eq!(NoneSeen.neighbour(DidRender), DidRender);
    /// ```
    pub fn neighbour(self, other: Self) -> Self {
        match (self, other) {
            // if either rendered, the parent group did too.
            (DidRender, _) => DidRender,
            (_, DidRender) => DidRender,
            // promote OnlyEmpty
            (OnlyEmpty, _) => OnlyEmpty,
            (_, OnlyEmpty) => OnlyEmpty,
            _ => NoneSeen,
        }
    }

    #[inline]
    pub fn should_render_tree(self) -> bool {
        self != OnlyEmpty
    }
}
