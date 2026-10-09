use crate::{LimitKind, NormalizeError, WorkControl};

pub(super) struct Output<'a> {
    text: String,
    maximum: usize,
    control: &'a WorkControl,
}

impl<'a> Output<'a> {
    pub(super) fn new(maximum: usize, control: &'a WorkControl) -> Self {
        Self {
            text: String::new(),
            maximum,
            control,
        }
    }

    pub(super) fn append(&mut self, text: &str) -> Result<(), NormalizeError> {
        self.check()?;
        self.text
            .len()
            .checked_add(text.len())
            .filter(|length| *length <= self.maximum)
            .ok_or(NormalizeError::LimitExceeded(LimitKind::Result))?;
        self.text.push_str(text);
        Ok(())
    }

    pub(super) fn word(&mut self, word: &str) -> Result<(), NormalizeError> {
        if !self.text.is_empty() && !self.text.ends_with(char::is_whitespace) {
            self.append(" ")?;
        }
        self.append(word)
    }

    pub(super) fn check(&self) -> Result<(), NormalizeError> {
        self.control.check()
    }

    pub(super) fn finish(self) -> String {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emission_accepts_exact_bytes_and_rejects_amplification_before_growth() {
        let control = WorkControl::default();
        let mut output = Output::new("üç dört".len(), &control);
        assert_eq!(output.word("üç"), Ok(()));
        assert_eq!(output.word("dört"), Ok(()));
        assert_eq!(
            output.append("a"),
            Err(NormalizeError::LimitExceeded(LimitKind::Result))
        );
        assert_eq!(output.finish(), "üç dört");
    }

    #[test]
    fn emission_observes_cancellation_after_work_has_begun() {
        let control = WorkControl::default();
        let mut output = Output::new(10000, &control);
        for _ in 0..100 {
            output.word("bir").unwrap();
        }
        control.cancel();
        assert_eq!(output.word("iki"), Err(NormalizeError::Cancelled));
    }
}
