use crate::choose::CondChecker;
use crate::prelude::*;
use citeproc_io::output::html::Html;
use citeproc_io::{DateOrRange, NumericValue, Reference};
use csl::locale::Locale;
use csl::style::{CslType, Position, Style, VariableForm};
use csl::terms::LocatorType;
use csl::variables::*;

use crate::disamb::FreeCond;

pub struct RefContext<'a, O: OutputFormat = Html> {
    pub format: &'a O,
    pub style: &'a Style,
    pub locale: &'a Locale,
    pub reference: &'a Reference,
    pub locator_type: Option<LocatorType>,
    pub position: Position,
    pub year_suffix: bool,
}

impl From<FreeCond> for Position {
    fn from(pos: FreeCond) -> Self {
        if pos.contains(FreeCond::IBID_WITH_LOCATOR) {
            Position::IbidWithLocator
        } else if pos.contains(FreeCond::IBID) {
            Position::Ibid
        } else if pos.contains(FreeCond::NEAR_NOTE) {
            Position::NearNote
        } else if pos.contains(FreeCond::FAR_NOTE) {
            Position::FarNote
        } else if pos.contains(FreeCond::SUBSEQUENT) {
            Position::Subsequent
        } else {
            // TODO: check this
            Position::First
        }
        // if not mentioned, it doesn't matter!
    }
}

impl<'c, O> RefContext<'c, O>
where
    O: OutputFormat,
{
    pub fn from_free_cond(
        fc: FreeCond,
        format: &'c O,
        style: &'c Style,
        locale: &'c Locale,
        reference: &'c Reference,
    ) -> Self {
        RefContext {
            format,
            style,
            locale,
            reference,
            locator_type: fc.to_loc_type(),
            position: Position::from(fc),
            year_suffix: fc.contains(FreeCond::YEAR_SUFFIX),
        }
    }
    pub fn get_ordinary(&self, var: Variable, form: VariableForm) -> Option<&str> {
        (match (var, form) {
            (Variable::Title, VariableForm::Short) => {
                self.reference.ordinary.get(&Variable::TitleShort)
            }
            _ => self.reference.ordinary.get(&var),
        })
        .map(|s| s.as_str())
    }
    pub fn get_number(&self, var: NumberVariable) -> Option<&NumericValue> {
        self.reference.number.get(&var)
    }
    pub fn has_variable(&self, var: AnyVariable) -> bool {
        match var {
            AnyVariable::Number(v) => match v {
                NumberVariable::Locator => self.locator_type.is_some(),
                NumberVariable::FirstReferenceNoteNumber => {
                    self.position.matches(Position::Subsequent)
                }
                NumberVariable::CitationNumber => self.style.bibliography.is_some(),
                _ => self.get_number(v).is_some(),
            },
            AnyVariable::Ordinary(v) => {
                match v {
                    // TODO: make Hereinafter a FreeCond
                    Variable::Hereinafter => unimplemented!("Hereinafter as a FreeCond"),
                    Variable::YearSuffix => unimplemented!("year_suffix: bool, in RefContext"),
                    _ => self.reference.ordinary.contains_key(&v),
                }
            }
            AnyVariable::Date(v) => self.reference.date.contains_key(&v),
            AnyVariable::Name(v) => self.reference.name.contains_key(&v),
        }
    }
}

impl<'c, O> CondChecker for RefContext<'c, O>
where
    O: OutputFormat,
{
    fn has_variable(&self, var: AnyVariable) -> bool {
        RefContext::has_variable(self, var)
    }
    fn is_numeric(&self, var: AnyVariable) -> bool {
        match &var {
            AnyVariable::Number(num) => self
                .reference
                .number
                .get(num)
                .map(|r| r.is_numeric())
                .unwrap_or(false),
            _ => false,
            // TODO: not very useful; implement for non-number variables (see CiteContext)
        }
    }
    fn csl_type(&self) -> &CslType {
        &self.reference.csl_type
    }
    fn get_date(&self, dvar: DateVariable) -> Option<&DateOrRange> {
        self.reference.date.get(&dvar)
    }
    fn position(&self) -> Position {
        self.position
    }
    fn is_disambiguate(&self) -> bool {
        false
    }
    fn style(&self) -> &Style {
        self.style
    }
}