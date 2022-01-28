use crate::{Project, Reaper};
use reaper_low::{
    copy_heap_buf_to_buf, create_heap_buf, load_pcm_source_state_from_buf,
    save_pcm_source_state_to_heap_buf,
};
use reaper_medium::{
    BorrowedPcmSource, DurationInSeconds, ExtGetPooledMidiIdResult, MidiImportBehavior,
    OwnedPcmSource, PcmSource, ReaperFunctionError,
};
use ref_cast::RefCast;
use std::borrow::Borrow;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// Pointer to a PCM source that's owned and managed by REAPER.
///
/// Whenever a function is called via `Deref`, a validation check will be done. If it doesn't
/// succeed, reaper-rs will panic (better than crashing).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
pub struct ReaperSource(PcmSource);

impl ReaperSource {
    pub fn new(raw: PcmSource) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> PcmSource {
        self.0
    }

    pub fn is_valid(&self) -> bool {
        Reaper::get().medium_reaper().validate_ptr(self.0)
    }

    pub fn is_valid_in_project(&self, project: Project) -> bool {
        Reaper::get()
            .medium_reaper()
            .validate_ptr_2(project.context(), self.0)
    }

    fn make_sure_is_valid(&self) {
        if !self.is_valid() {
            panic!("PCM source pointer is not valid anymore in REAPER")
        }
    }
}

impl AsRef<BorrowedSource> for ReaperSource {
    fn as_ref(&self) -> &BorrowedSource {
        self.make_sure_is_valid();
        BorrowedSource::ref_cast(unsafe { self.0.as_ref() })
    }
}

impl AsMut<BorrowedSource> for ReaperSource {
    fn as_mut(&mut self) -> &mut BorrowedSource {
        self.make_sure_is_valid();
        BorrowedSource::ref_cast_mut(unsafe { self.0.as_mut() })
    }
}

impl Deref for ReaperSource {
    type Target = BorrowedSource;

    fn deref(&self) -> &BorrowedSource {
        self.as_ref()
    }
}

#[derive(Eq, PartialEq, Hash, Debug, RefCast)]
#[repr(transparent)]
pub struct BorrowedSource(BorrowedPcmSource);

impl BorrowedSource {
    pub fn file_name(&self) -> Option<PathBuf> {
        self.0.get_file_name(|path| path.map(|p| p.to_owned()))
    }

    pub fn r#type(&self) -> String {
        self.0.get_type(|t| t.to_string())
    }

    pub fn length(&self) -> Result<DurationInSeconds, ReaperFunctionError> {
        self.0.get_length()
    }

    pub fn duplicate(&self) -> Option<OwnedSource> {
        let raw_duplicate = self.0.duplicate()?;
        Some(OwnedSource::new(raw_duplicate))
    }

    // We return a medium-level source because at this point we don't know if the parent is a
    // REAPER-managed source or not.
    pub fn parent_source(&self) -> Option<PcmSource> {
        let raw = self.0.get_source()?;
        Some(raw)
    }

    // We return a medium-level source because at this point we don't know if the root is a
    // REAPER-managed source or not.
    pub fn root_source(&self) -> PcmSource {
        let mut source_ptr = self.0.as_ptr();
        loop {
            let source = unsafe { source_ptr.as_ref() };
            if let Some(parent) = source.get_source() {
                source_ptr = parent;
            } else {
                return source_ptr;
            }
        }
    }

    pub fn pooled_midi_id(&self) -> Result<ExtGetPooledMidiIdResult, ReaperFunctionError> {
        self.0.ext_get_pooled_midi_id()
    }

    pub fn export_to_file(&self, file: &Path) -> Result<(), ReaperFunctionError> {
        self.0.ext_export_to_file(file)
    }

    pub fn state_chunk(&self) -> String {
        let heap_buf = create_heap_buf();
        let size = unsafe { save_pcm_source_state_to_heap_buf(self.0.as_ptr().to_raw(), heap_buf) };
        let mut buffer = vec![0u8; size as usize];
        unsafe { copy_heap_buf_to_buf(heap_buf, buffer.as_mut_ptr()) };
        // I think it's safe to assume that the content written to the buffer is made up by multiple
        // segments, each of which is a proper UTF-8-encoded line (not containing newlines or
        // carriage returns). Each segment is separated by a nul byte. So if we convert each nul
        // byte to a newline, we should obtain a proper UTF-8-encoded string (which doesn't contain
        // a trailing nul byte)!
        for b in &mut buffer {
            if *b == b'\0' {
                *b = b'\n';
            }
        }
        unsafe { String::from_utf8_unchecked(buffer) }
    }

    pub fn set_state_chunk(&mut self, chunk: String) {
        let mut buffer: Vec<u8> = chunk.into();
        for b in &mut buffer {
            if *b == b'\n' {
                *b = b'\0';
            }
        }
        unsafe {
            load_pcm_source_state_from_buf(
                self.0.as_ptr().to_raw(),
                buffer.as_mut_ptr(),
                buffer.len() as _,
            );
        }
    }
}

/// Owned PCM source.
#[derive(Debug)]
#[repr(transparent)]
pub struct OwnedSource(OwnedPcmSource);

impl OwnedSource {
    pub fn new(raw: OwnedPcmSource) -> Self {
        Self(raw)
    }

    pub fn into_raw(self) -> OwnedPcmSource {
        self.0
    }

    pub fn from_file(
        file: &Path,
        import_behavior: MidiImportBehavior,
    ) -> Result<Self, &'static str> {
        let raw = Reaper::get()
            .medium_reaper()
            .pcm_source_create_from_file_ex(file, import_behavior)
            .map_err(|_| "couldn't create PCM source")?;
        Ok(Self(raw))
    }
}

impl AsRef<BorrowedSource> for OwnedSource {
    fn as_ref(&self) -> &BorrowedSource {
        BorrowedSource::ref_cast(self.0.as_ref())
    }
}

impl Borrow<BorrowedSource> for OwnedSource {
    fn borrow(&self) -> &BorrowedSource {
        self.as_ref()
    }
}

impl ToOwned for BorrowedSource {
    type Owned = OwnedSource;

    fn to_owned(&self) -> OwnedSource {
        self.duplicate().expect("source not cloneable")
    }
}

impl Deref for OwnedSource {
    type Target = BorrowedSource;

    fn deref(&self) -> &BorrowedSource {
        self.as_ref()
    }
}

impl Clone for OwnedSource {
    fn clone(&self) -> OwnedSource {
        self.duplicate()
            .expect("this source doesn't support duplication")
    }
}
