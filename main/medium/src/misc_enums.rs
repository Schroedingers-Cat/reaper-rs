use crate::{concat_c_strs, MidiDeviceId, ReaperStringArg};
use c_str_macro::c_str;
use helgoboss_midi::{U14, U7};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::borrow::Cow;
use std::ffi::CStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackFxChainType {
    NormalFxChain,
    /// On the master track this corresponds to the monitoring FX chain
    InputFxChain,
}

// TODO-medium Maybe better implement this as normal pub(crate) method because it's an
// implementation detail
impl From<TrackFxChainType> for bool {
    fn from(t: TrackFxChainType) -> Self {
        t == TrackFxChainType::InputFxChain
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MasterTrackBehavior {
    ExcludeMasterTrack,
    IncludeMasterTrack,
}

impl From<MasterTrackBehavior> for bool {
    fn from(v: MasterTrackBehavior) -> Self {
        v == MasterTrackBehavior::IncludeMasterTrack
    }
}

// TODO-medium Wait for jf to explain the meaning of this
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UndoHint {
    Normal,
    IsUndo,
}

impl From<UndoHint> for bool {
    fn from(v: UndoHint) -> Self {
        v == UndoHint::IsUndo
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueChange<T: Copy> {
    Absolute(T),
    Relative(T),
}

impl<T: Copy> ValueChange<T> {
    pub(crate) fn value(&self) -> T {
        use ValueChange::*;
        match self {
            Absolute(v) => *v,
            Relative(v) => *v,
        }
    }

    pub(crate) fn is_relative(&self) -> bool {
        matches!(self, ValueChange::Relative(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UndoBehavior {
    OmitUndoPoint,
    AddUndoPoint,
}

impl From<UndoBehavior> for bool {
    fn from(h: UndoBehavior) -> Self {
        h == UndoBehavior::AddUndoPoint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferBehavior {
    Copy,
    Move,
}

impl From<TransferBehavior> for bool {
    fn from(t: TransferBehavior) -> Self {
        t == TransferBehavior::Move
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackDefaultsBehavior {
    OmitDefaultEnvAndFx,
    AddDefaultEnvAndFx,
}

impl From<TrackDefaultsBehavior> for bool {
    fn from(v: TrackDefaultsBehavior) -> Self {
        v == TrackDefaultsBehavior::AddDefaultEnvAndFx
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GangBehavior {
    DenyGang,
    AllowGang,
}

impl From<GangBehavior> for bool {
    fn from(v: GangBehavior) -> Self {
        v == GangBehavior::AllowGang
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive)]
#[repr(i32)]
pub enum RecordArmState {
    Unarmed = 0,
    Armed = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive)]
#[repr(i32)]
pub enum FxShowFlag {
    HideChain = 0,
    ShowChain = 1,
    HideFloatingWindow = 2,
    ShowFloatingWindow = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive)]
#[repr(i32)]
pub enum TrackSendDirection {
    Receive = -1,
    Send = 0,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive)]
#[repr(i32)]
pub enum TrackSendCategory {
    Receive = -1,
    Send = 0,
    HardwareOutput = 1,
}

impl From<TrackSendDirection> for TrackSendCategory {
    fn from(v: TrackSendDirection) -> Self {
        use TrackSendDirection::*;
        match v {
            Receive => TrackSendCategory::Receive,
            Send => TrackSendCategory::Send,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StuffMidiMessageTarget {
    VirtualMidiKeyboardQueue,
    MidiAsControlInputQueue,
    VirtualMidiKeyboardQueueOnCurrentChannel,
    MidiOutputDevice(MidiDeviceId),
}

impl From<StuffMidiMessageTarget> for i32 {
    fn from(t: StuffMidiMessageTarget) -> Self {
        use StuffMidiMessageTarget::*;
        match t {
            VirtualMidiKeyboardQueue => 0,
            MidiAsControlInputQueue => 1,
            VirtualMidiKeyboardQueueOnCurrentChannel => 2,
            MidiOutputDevice(id) => 16 + id.0 as i32,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackFxRef {
    NormalFxChain(u32),
    InputFxChain(u32),
}

// Converts directly to the i32 value that is expected by low-level track-FX related functions
impl From<TrackFxRef> for i32 {
    fn from(v: TrackFxRef) -> Self {
        use TrackFxRef::*;
        let positive = match v {
            InputFxChain(idx) => 0x1000000 + idx,
            NormalFxChain(idx) => idx,
        };
        positive as i32
    }
}

// Converts from a value returned by low-level track-FX related functions turned into u32.
impl From<u32> for TrackFxRef {
    fn from(v: u32) -> Self {
        use TrackFxRef::*;
        if v >= 0x1000000 {
            InputFxChain(v - 0x1000000)
        } else {
            NormalFxChain(v)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive)]
#[repr(i32)]
pub enum TrackFxAddByNameBehavior {
    Add = -1,
    Query = 0,
    AddIfNotFound = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionValueChange {
    AbsoluteLowRes(U7),
    AbsoluteHighRes(U14),
    Relative1(U7),
    Relative2(U7),
    Relative3(U7),
}

// TODO-medium If those are not extensions and only the complete DLL is the extension, name this
//  differently.
#[derive(Clone, Debug)]
pub enum ExtensionType<'a> {
    Api(Cow<'a, CStr>),
    ApiDef(Cow<'a, CStr>),
    HookCommand,
    HookPostCommand,
    HookCommand2,
    ToggleAction,
    ActionHelp,
    CommandId,
    CommandIdLookup,
    GAccel,
    CSurfInst,
    Custom(Cow<'a, CStr>),
}

impl<'a> ExtensionType<'a> {
    pub fn api(func_name: impl Into<ReaperStringArg<'a>>) -> Self {
        Self::Api(func_name.into().into_cow())
    }

    pub fn api_def(func_def: impl Into<ReaperStringArg<'a>>) -> Self {
        Self::ApiDef(func_def.into().into_cow())
    }

    pub fn custom(key: impl Into<ReaperStringArg<'a>>) -> Self {
        Self::Custom(key.into().into_cow())
    }
}

impl<'a> From<ExtensionType<'a>> for Cow<'a, CStr> {
    fn from(value: ExtensionType<'a>) -> Self {
        use ExtensionType::*;
        match value {
            GAccel => c_str!("gaccel").into(),
            CSurfInst => c_str!("csurf_inst").into(),
            Api(func_name) => concat_c_strs(c_str!("API_"), func_name.as_ref()).into(),
            ApiDef(func_def) => concat_c_strs(c_str!("APIdef_"), func_def.as_ref()).into(),
            HookCommand => c_str!("hookcommand").into(),
            HookPostCommand => c_str!("hookpostcommand").into(),
            HookCommand2 => c_str!("hookcommand2").into(),
            ToggleAction => c_str!("toggleaction").into(),
            ActionHelp => c_str!("action_help").into(),
            CommandId => c_str!("command_id").into(),
            CommandIdLookup => c_str!("command_id_lookup").into(),
            Custom(k) => k,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackRef {
    MasterTrack,
    NormalTrack(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum InputMonitoringMode {
    Off = 0,
    Normal = 1,
    /// Tape style
    NotWhenPlaying = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectRef {
    Current,
    CurrentlyRendering,
    Tab(u32),
}