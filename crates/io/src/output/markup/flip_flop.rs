// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2019 Corporation for Digital Scholarship

use super::InlineElement;
use super::QuoteType;
use crate::output::micro_html::MicroNode;
use crate::output::FormatCmd;
use csl::{FontStyle, FontVariant, FontWeight, Formatting};

#[derive(Default, Debug, Clone)]
pub struct FlipFlopState {
    in_emph: bool,
    emph: FontStyle,
    in_strong: bool,
    in_small_caps: bool,
    in_outer_quotes: bool,
}

impl FlipFlopState {
    pub fn from_formatting(f: Formatting) -> Self {
        FlipFlopState {
            emph: f.font_style.unwrap_or_default(),
            in_emph: f.font_style == Some(FontStyle::Italic)
                || f.font_style == Some(FontStyle::Oblique),
            in_strong: f.font_weight == Some(FontWeight::Bold),
            in_small_caps: f.font_variant == Some(FontVariant::SmallCaps),
            // TODO: quotes
            in_outer_quotes: false,
        }
    }
    pub fn flip_flop_inlines(&self, inlines: &[InlineElement]) -> Vec<InlineElement> {
        inlines
            .iter()
            .map(|inl| flip_flop(inl, self).unwrap_or_else(|| inl.clone()))
            .collect()
    }
}

fn flip_flop(inline: &InlineElement, state: &FlipFlopState) -> Option<InlineElement> {
    use super::InlineElement::*;
    match inline {
        Micro(nodes) => {
            let subs = flip_flop_nodes(nodes, state);
            Some(Micro(subs))
        }
        Formatted(ils, f) => {
            let mut flop = state.clone();
            let mut new_f = *f;
            if let Some(fs) = f.font_style {
                let samey = fs == state.emph;
                if samey {
                    new_f.font_style = None;
                }
                flop.in_emph = match fs {
                    FontStyle::Italic | FontStyle::Oblique => true,
                    _ => false,
                };
                flop.emph = fs;
            }
            if let Some(fw) = f.font_weight {
                let want = fw == FontWeight::Bold;
                if flop.in_strong == want && want {
                    new_f.font_weight = None;
                }
                flop.in_strong = want;
            }
            if let Some(fv) = f.font_variant {
                let want_small_caps = fv == FontVariant::SmallCaps;
                if flop.in_small_caps == want_small_caps {
                    new_f.font_variant = None;
                }
                flop.in_small_caps = want_small_caps;
            }
            let subs = flop.flip_flop_inlines(ils);
            Some(Formatted(subs, new_f))
        }

        Quoted(ref _q, ref ils) => {
            let mut flop = state.clone();
            flop.in_outer_quotes = !flop.in_outer_quotes;
            let subs = flop.flip_flop_inlines(ils);
            if !state.in_outer_quotes {
                Some(Quoted(QuoteType::SingleQuote, subs))
            } else {
                Some(Quoted(QuoteType::DoubleQuote, subs))
            }
        }

        Anchor {
            title,
            url,
            content,
        } => {
            let subs = state.flip_flop_inlines(content);
            Some(Anchor {
                title: title.clone(),
                url: url.clone(),
                content: subs,
            })
        }

        _ => None,
    }

    // a => a
}
fn flip_flop_nodes(nodes: &[MicroNode], state: &FlipFlopState) -> Vec<MicroNode> {
    nodes
        .iter()
        .map(|nod| flip_flop_node(nod, state).unwrap_or_else(|| nod.clone()))
        .collect()
}

fn flip_flop_node(node: &MicroNode, state: &FlipFlopState) -> Option<MicroNode> {
    match node {
        MicroNode::Formatted(ref nodes, cmd) => {
            let mut flop = state.clone();
            match cmd {
                FormatCmd::FontStyleItalic => {
                    flop.in_emph = !flop.in_emph;
                    let subs = flip_flop_nodes(nodes, &flop);
                    if state.in_emph {
                        Some(MicroNode::Formatted(subs, FormatCmd::FontStyleNormal))
                    } else {
                        Some(MicroNode::Formatted(subs, *cmd))
                    }
                }
                FormatCmd::FontWeightBold => {
                    flop.in_strong = !flop.in_strong;
                    let subs = flip_flop_nodes(nodes, &flop);
                    if state.in_strong {
                        Some(MicroNode::Formatted(subs, FormatCmd::FontWeightNormal))
                    } else {
                        Some(MicroNode::Formatted(subs, *cmd))
                    }
                }
                FormatCmd::FontVariantSmallCaps => {
                    flop.in_small_caps = !flop.in_small_caps;
                    let subs = flip_flop_nodes(nodes, &flop);
                    if state.in_small_caps {
                        Some(MicroNode::Formatted(subs, FormatCmd::FontVariantNormal))
                    } else {
                        Some(MicroNode::Formatted(subs, *cmd))
                    }
                }
                // i.e. sup and sub
                _ => {
                    let subs = flip_flop_nodes(nodes, state);
                    Some(MicroNode::Formatted(subs, *cmd))
                }
            }
        }
        MicroNode::Text(_) => None,
        MicroNode::NoCase(ref nodes) => {
            let subs = flip_flop_nodes(nodes, state);
            Some(MicroNode::NoCase(subs))
        }
    }
}
