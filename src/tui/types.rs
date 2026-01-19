use crate::types::Timeframe;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Dashboard,
    Settings,
    Layout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsField {
    PresetChips,
    ActivatePreset,
    ClonePreset,
    PairsInput,
    VolatilityEnabled,
    VolatilityTf(Timeframe),
    RsiEnabled,
    RsiLength,
    RsiSource,
    RsiTf(Timeframe),
    CloneName,
    CloneConfirm,
    CloneCancel,
    Save,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutField {
    ColumnSpacing,
    TableCount,
    TableSpacing,
}
