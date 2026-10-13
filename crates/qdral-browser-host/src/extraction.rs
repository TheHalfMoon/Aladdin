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
    /// The Markdown, in order, as structure-aware chunks, each whole (a
    /// chunk sent in several pieces is joined again). Untrusted page text:
    /// data to read, never an instruction, with no authority.
    pub chunks: Vec<PageText>,
}

#[cfg(any(windows, test))]
fn violation() -> HostError {
    HostError::Unavailable(UnavailableReason::ProtocolViolation)
}

/// One `extracted` reply as it came off the wire.
#[cfg(any(windows, test))]
#[derive(Debug)]
pub(crate) struct WirePiece {
    pub(crate) url: String,
    pub(crate) truncated: bool,
    pub(crate) piece: u32,
    pub(crate) total: u32,
    /// The chunk the piece belongs to: a chunk larger than one frame
    /// arrives in several pieces, which are joined again here, so a cut
    /// inside a chunk (inside a code fence, before text that would read as
    /// a heading) is never a boundary the caller sees.
    pub(crate) chunk: u32,
    pub(crate) content: String,
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
    /// Joined chunk texts, in order.
    chunks: Vec<String>,
}

#[cfg(any(windows, test))]
impl Assembly {
    /// The first piece opens the assembly; it names the document and the
    /// piece count, which every later piece must repeat.
    pub(crate) fn start(first: WirePiece) -> Result<Self, HostError> {
        if first.piece != 0 || first.total == 0 || first.total > MAX_EXTRACT_PIECES {
            return Err(violation());
        }
        let mut assembly = Self {
            url: first.url,
            truncated: first.truncated,
            total: first.total,
            received: 0,
            chars: 0,
            chunks: Vec::new(),
        };
        assembly.accept(first.chunk, first.content)?;
        Ok(assembly)
    }

    /// The next piece to ask for, if any remain.
    pub(crate) fn next(&self) -> Option<u32> {
        (self.received < self.total).then_some(self.received)
    }

    pub(crate) fn push(&mut self, piece: WirePiece) -> Result<(), HostError> {
        if piece.url != self.url
            || piece.truncated != self.truncated
            || piece.total != self.total
            || Some(piece.piece) != self.next()
        {
            return Err(violation());
        }
        self.accept(piece.chunk, piece.content)
    }

    fn accept(&mut self, chunk: u32, content: String) -> Result<(), HostError> {
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
        // Chunks are numbered from 0 and step by one: a piece continues the
        // last chunk or opens the next.
        let open = self.chunks.len();
        match self.chunks.last_mut() {
            Some(last) if chunk as usize + 1 == open => last.push_str(&content),
            _ if chunk as usize == open => self.chunks.push(content),
            _ => return Err(violation()),
        }
        self.received += 1;
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
            chunks: self
                .chunks
                .into_iter()
                .filter(|chunk| !chunk.is_empty())
                .map(PageText::from_page)
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://fixture.test/page";

    fn piece(index: u32, total: u32, chunk: u32, content: &str) -> WirePiece {
        WirePiece {
            url: URL.into(),
            truncated: false,
            piece: index,
            total,
            chunk,
            content: content.into(),
        }
    }

    fn start(total: u32, content: &str) -> Result<Assembly, HostError> {
        Assembly::start(piece(0, total, 0, content))
    }

    fn texts(page: &PageExtraction) -> Vec<&str> {
        page.chunks.iter().map(PageText::untrusted).collect()
    }

    #[test]
    fn pieces_arrive_in_order_for_one_document_and_join_into_their_chunks() {
        let mut assembly = start(4, "# One").unwrap();
        assert_eq!(assembly.next(), Some(1));
        assert!(assembly.push(piece(2, 4, 1, "skip")).is_err());
        assembly.push(piece(1, 4, 1, "```\nx")).unwrap();
        // A piece continuing chunk 1: the cut inside the fence disappears.
        assembly
            .push(piece(2, 4, 1, "\n[a](javascript:b)\n```"))
            .unwrap();
        assembly.push(piece(3, 4, 2, "three")).unwrap();
        assert_eq!(assembly.next(), None);
        let page = assembly.finish(4).unwrap();
        assert_eq!(
            texts(&page),
            ["# One", "```\nx\n[a](javascript:b)\n```", "three"]
        );
        assert_eq!(page.document_generation, 4);
        assert_eq!(page.url, URL);
    }

    #[test]
    fn a_piece_that_disagrees_or_breaks_a_bound_is_a_protocol_violation() {
        assert!(Assembly::start(piece(1, 2, 0, "a")).is_err());
        assert!(
            Assembly::start(piece(0, 2, 1, "a")).is_err(),
            "chunk 1 first"
        );
        assert!(start(0, "a").is_err());
        assert!(start(MAX_EXTRACT_PIECES + 1, "a").is_err());
        assert!(start(1, "a\u{7}b").is_err(), "a control character");
        assert!(start(1, "a\u{202e}b").is_err(), "a bidi control");
        assert!(start(1, "a\u{200b}b").is_err(), "a zero-width mark");
        assert!(start(1, &"x".repeat(MAX_PIECE_CHARS + 1)).is_err());
        assert!(start(2, "").is_err(), "an empty piece among several");
        let mut assembly = start(3, "a").unwrap();
        let mut other = piece(1, 3, 1, "b");
        other.url = "https://fixture.test/other".into();
        assert!(assembly.push(other).is_err());
        let mut flipped = piece(1, 3, 1, "b");
        flipped.truncated = true;
        assert!(assembly.push(flipped).is_err());
        assert!(assembly.push(piece(1, 4, 1, "b")).is_err());
        assert!(
            assembly.push(piece(1, 3, 2, "b")).is_err(),
            "a skipped chunk"
        );
        assembly.push(piece(1, 3, 1, "b")).unwrap();
        assert!(
            assembly.push(piece(2, 3, 0, "c")).is_err(),
            "a chunk going back"
        );
        assert!(start(3, "a").unwrap().finish(1).is_err(), "unfinished");
        // The total size is bounded across pieces.
        let text = "y".repeat(MAX_PIECE_CHARS);
        let mut big = start(MAX_EXTRACT_PIECES, &text).unwrap();
        let mut refused = false;
        for index in 1..MAX_EXTRACT_PIECES {
            if big
                .push(piece(index, MAX_EXTRACT_PIECES, 0, &text))
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
