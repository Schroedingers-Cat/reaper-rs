use crate::{Project, Reaper, Take};
use reaper_low::raw::PCM_source;
use reaper_medium::{
    BorrowedPcmSource, DurationInSeconds, ExtGetPooledMidiIdResult, MidiImportBehavior,
    OwnedPcmSource, PcmSource, ReaperFunctionError,
};
use std::borrow::Borrow;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

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

    pub unsafe fn as_ref(&self) -> &BorrowedSource {
        &*(self.0.as_ptr() as *const BorrowedSource)
    }

    fn make_sure_is_valid(&self) {
        if !self.is_valid() {
            panic!("PCM source pointer is not valid anymore in REAPER")
        }
    }
}

impl Deref for ReaperSource {
    type Target = BorrowedSource;

    fn deref(&self) -> &BorrowedSource {
        self.make_sure_is_valid();
        unsafe { self.as_ref() }
    }
}

#[derive(Eq, PartialEq, Hash, Debug)]
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
}

/// Owned PCM source.
#[derive(Debug)]
#[repr(transparent)]
pub struct OwnedSource(OwnedPcmSource);

impl OwnedSource {
    pub fn new(raw: OwnedPcmSource) -> Self {
        Self(raw)
    }

    pub fn from_file(
        file: &Path,
        import_behavior: MidiImportBehavior,
    ) -> Result<Self, &'static str> {
        unsafe {
            let raw = Reaper::get()
                .medium_reaper()
                .pcm_source_create_from_file_ex(file, import_behavior)
                .map_err(|_| "couldn't create PCM source")?;
            Ok(Self(raw))
        }
    }
}

impl Deref for OwnedSource {
    type Target = BorrowedSource;

    fn deref(&self) -> &BorrowedSource {
        unsafe { &*(self.0.as_ptr().as_ptr() as *const BorrowedSource) }
    }
}

impl Clone for OwnedSource {
    fn clone(&self) -> OwnedSource {
        self.duplicate()
            .expect("this source doesn't support duplication")
    }
}
