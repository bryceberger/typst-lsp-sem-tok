use typst::ide::Tag;

use strum::{EnumIter, IntoStaticStr};

pub trait ToSemanticToken {
    fn to_name(&self) -> &'static str;
    fn to_idx(&self) -> u32;
}

impl ToSemanticToken for TypstSemanticToken {
    fn to_name(&self) -> &'static str {
        self.into()
    }
    fn to_idx(&self) -> u32 {
        *self as u32
    }
}

impl ToSemanticToken for Tag {
    fn to_name(&self) -> &'static str {
        let converted: TypstSemanticToken = (*self).into();
        converted.to_name()
    }
    fn to_idx(&self) -> u32 {
        let converted: TypstSemanticToken = (*self).into();
        converted.to_idx()
    }
}

impl ToSemanticToken for Option<Tag> {
    fn to_name(&self) -> &'static str {
        let converted: TypstSemanticToken = (*self).into();
        converted.to_name()
    }
    fn to_idx(&self) -> u32 {
        let converted: TypstSemanticToken = (*self).into();
        converted.to_idx()
    }
}

/// Copied from typst to derive strum
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter, IntoStaticStr)]
#[repr(u32)]
pub enum TypstSemanticToken {
    Comment,
    Punctuation,
    Escape,
    Strong,
    Emph,
    Link,
    Raw,
    Label,
    Ref,
    Heading,
    ListMarker,
    ListTerm,
    MathDelimiter,
    MathOperator,
    Keyword,
    Operator,
    Number,
    String,
    Function,
    Interpolated,
    Error,
    None,
}

impl From<Option<Tag>> for TypstSemanticToken {
    fn from(value: Option<Tag>) -> Self {
        match value {
            Some(val) => val.into(),
            None => Self::None,
        }
    }
}

impl From<Tag> for TypstSemanticToken {
    fn from(value: Tag) -> Self {
        use Tag::*;
        match value {
            Comment => Self::Comment,
            Punctuation => Self::Punctuation,
            Escape => Self::Escape,
            Strong => Self::Strong,
            Emph => Self::Emph,
            Link => Self::Link,
            Raw => Self::Raw,
            Label => Self::Label,
            Ref => Self::Ref,
            Heading => Self::Heading,
            ListMarker => Self::ListMarker,
            ListTerm => Self::ListTerm,
            MathDelimiter => Self::MathDelimiter,
            MathOperator => Self::MathOperator,
            Keyword => Self::Keyword,
            Operator => Self::Operator,
            Number => Self::Number,
            String => Self::String,
            Function => Self::Function,
            Interpolated => Self::Interpolated,
            Error => Self::Error,
        }
    }
}
