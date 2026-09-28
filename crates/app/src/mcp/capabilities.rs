use mcp_authorization::Capability;

/// Anyone — read others' public prayers.
pub struct ReadPublic;
impl Capability for ReadPublic {
    const NAME: &'static str = "read_public";
}

/// Authenticated — read all your own prayers at any visibility.
pub struct ReadOwn;
impl Capability for ReadOwn {
    const NAME: &'static str = "read_own";
}

/// Authenticated — create, edit, delete, change visibility (own prayers only).
pub struct WriteOwn;
impl Capability for WriteOwn {
    const NAME: &'static str = "write_own";
}

/// Authenticated — "praying now" (+1) on any prayer you can see.
pub struct Pray;
impl Capability for Pray {
    const NAME: &'static str = "pray";
}

/// Authenticated + group member — read prayers shared to your groups.
pub struct ReadGroup;
impl Capability for ReadGroup {
    const NAME: &'static str = "read_group";
}

/// Authenticated + group member — share one of your prayers to a group.
pub struct ShareToGroup;
impl Capability for ShareToGroup {
    const NAME: &'static str = "share_to_group";
}

/// Authenticated — create groups and invite others.
pub struct ManageGroups;
impl Capability for ManageGroups {
    const NAME: &'static str = "manage_groups";
}
