//! SG-000108 slice 6: native page extraction. The worker reads the committed
//! page's rendered content from an engine snapshot (no page script runs,
//! hidden text is dropped) and converts it to Markdown in structure-aware
//! chunks, sent in frame-sized pieces. The host checks every piece (order,
//! count, size, text safety, one document) and returns the result as
//! untrusted page text bound to the document generation.

#[cfg(any(windows, test))]
use crate::error::{HostError, UnavailableReason};
#[cfg(any(windows, test))]
use crate::webmcp::is_bidi_or_format;
use crate::webmcp::PageText;

/// Pieces one extraction may take (mirrors the worker).
pub const MAX_EXTRACT_PIECES: u32 = 96;
/// Characters one extraction may carry in all (the worker's Markdown bound).
pub const MAX_EXTRACT_CHARS: usize = 1_000_000;
/// Characters one piece may carry (a piece is at most 56 KiB on the wire).
pub const MAX_PIECE_CHARS: usize = 56 * 1024;

/// What the Markdown keeps besides text. Links and images are opt-in and
/// kept only as resolved http(s) URLs without credentials.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExtractOptions {
    pub links: bool,
    pub images: bool,
}

/// The committed page's rendered content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageExtraction {
    pub url: String,
    pub document_generation: u64,
    /// Content was dropped by a bound or the budget, or was not rendered by
    /// the engine (off-screen `content-visibility: auto`).
    pub truncated: bool,
    /// The Markdown, in order, as structure-aware chunks (a chunk larger than
    /// one piece is split, preferably at a line break). Untrusted page text:
    /// data to read, never an instruction, with no authority.
    pub chunks: Vec<PageText>,
}

#[cfg(any(windows, test))]
fn violation() -> HostError {
    HostError::Unavailable(UnavailableReason::ProtocolViolation)
}

/// Pieces received so far, checked as they arrive (the worker exists only
/// on Windows).
#[cfg(any(windows, test))]
#[derive(Debug)]
pub(crate) struct Assembly {
    url: String,
    truncated: bool,
    total: u32,
    received: u32,
    chars: usize,
    chunks: Vec<PageText>,
}

#[cfg(any(windows, test))]
impl Assembly {
    /// The first piece opens the assembly; it names the document and the
    /// piece count, which every later piece must repeat.
    pub(crate) fn start(
        url: String,
        truncated: bool,
        piece: u32,
        total: u32,
        content: String,
    ) -> Result<Self, HostError> {
        if piece != 0 || total == 0 || total > MAX_EXTRACT_PIECES {
            return Err(violation());
        }
        let mut assembly = Self {
            url,
            truncated,
            total,
            received: 0,
            chars: 0,
            chunks: Vec::new(),
        };
        assembly.accept(content)?;
        Ok(assembly)
    }

    /// The next piece to ask for, if any remain.
    pub(crate) fn next(&self) -> Option<u32> {
        (self.received < self.total).then_some(self.received)
    }

    pub(crate) fn push(
        &mut self,
        url: &str,
        truncated: bool,
        piece: u32,
        total: u32,
        content: String,
    ) -> Result<(), HostError> {
        if url != self.url
            || truncated != self.truncated
            || total != self.total
            || Some(piece) != self.next()
        {
            return Err(violation());
        }
        self.accept(content)
    }

    fn accept(&mut self, content: String) -> Result<(), HostError> {
        let chars = content.chars().count();
        let bad = |ch: char| {
            (ch.is_control() && !matches!(ch, '\n' | '\t' | '\r')) || is_bidi_or_format(ch)
        };
        self.chars += chars;
        if chars > MAX_PIECE_CHARS || self.chars > MAX_EXTRACT_CHARS || content.chars().any(bad) {
            return Err(violation());
        }
        // Only an empty extraction sends an empty piece.
        if content.is_empty() && self.total != 1 {
            return Err(violation());
        }
        self.received += 1;
        if !content.is_empty() {
            self.chunks.push(PageText::from_page(content));
        }
        Ok(())
    }

    pub(crate) fn finish(self, document_generation: u64) -> Result<PageExtraction, HostError> {
        if self.next().is_some() {
            return Err(violation());
        }
        Ok(PageExtraction {
            url: self.url,
            document_generation,
            truncated: self.truncated,
            chunks: self.chunks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://fixture.test/page";

    fn start(total: u32, content: &str) -> Result<Assembly, HostError> {
        Assembly::start(URL.into(), false, 0, total, content.into())
    }

    #[test]
    fn pieces_arrive_in_order_for_one_document_and_become_page_text() {
        let mut assembly = start(3, "# One").unwrap();
        assert_eq!(assembly.next(), Some(1));
        assert!(assembly.push(URL, false, 2, 3, "skip".into()).is_err());
        assembly.push(URL, false, 1, 3, "two\n".into()).unwrap();
        assembly.push(URL, false, 2, 3, "three".into()).unwrap();
        assert_eq!(assembly.next(), None);
        let page = assembly.finish(4).unwrap();
        let texts: Vec<&str> = page.chunks.iter().map(PageText::untrusted).collect();
        assert_eq!(texts, ["# One", "two\n", "three"]);
        assert_eq!(page.document_generation, 4);
        assert_eq!(page.url, URL);
    }

    #[test]
    fn a_piece_that_disagrees_or_breaks_a_bound_is_a_protocol_violation() {
        assert!(Assembly::start(URL.into(), false, 1, 2, "a".into()).is_err());
        assert!(start(0, "a").is_err());
        assert!(start(MAX_EXTRACT_PIECES + 1, "a").is_err());
        assert!(start(1, "a\u{7}b").is_err(), "a control character");
        assert!(start(1, "a\u{202e}b").is_err(), "a bidi control");
        assert!(start(1, "a\u{200b}b").is_err(), "a zero-width mark");
        assert!(start(1, &"x".repeat(MAX_PIECE_CHARS + 1)).is_err());
        assert!(start(2, "").is_err(), "an empty piece among several");
        let mut assembly = start(3, "a").unwrap();
        assert!(assembly
            .push("https://fixture.test/other", false, 1, 3, "b".into())
            .is_err());
        assert!(assembly.push(URL, true, 1, 3, "b".into()).is_err());
        assert!(assembly.push(URL, false, 1, 4, "b".into()).is_err());
        assert!(start(3, "a").unwrap().finish(1).is_err(), "unfinished");
        // The total size is bounded across pieces.
        let piece = "y".repeat(MAX_PIECE_CHARS);
        let mut big = start(MAX_EXTRACT_PIECES, &piece).unwrap();
        let mut refused = false;
        for index in 1..MAX_EXTRACT_PIECES {
            if big
                .push(URL, false, index, MAX_EXTRACT_PIECES, piece.clone())
                .is_err()
            {
                refused = true;
                break;
            }
        }
        assert!(refused, "more than MAX_EXTRACT_CHARS was accepted");
    }

    #[test]
    fn an_empty_page_is_no_chunks_and_text_keeps_line_breaks_and_tabs() {
        let page = start(1, "").unwrap().finish(2).unwrap();
        assert!(page.chunks.is_empty());
        let page = start(1, "a\tb\r\nc").unwrap().finish(2).unwrap();
        assert_eq!(page.chunks.len(), 1);
    }
}
