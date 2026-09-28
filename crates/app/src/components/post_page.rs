use leptos::prelude::*;
use leptosbook::use_folio_lock;
use thanksgivings_core::{
    Group, Post, PostAction, PostState, PostTransition, PrayerCount, Viewer, Visibility,
};
use time::{format_description, OffsetDateTime};

use crate::pages::book::{pray_for, set_post_state, share_prayer_to_group};

#[component]
pub fn PostPage(
    post: Post,
    /// How the person looking at this entry relates to it; decides which
    /// actions (`PostAction::allowed`) are offered.
    viewer: Viewer,
    /// The viewer's groups, offered as share targets.
    #[prop(optional, into)] groups: Signal<Vec<Group>>,
    /// Whether the viewer owns this prayer and may share it.
    #[prop(optional, into)] can_share: Signal<bool>,
    /// Called after this entry moves to a new state, so the book can reload.
    #[prop(optional)] on_state_change: Option<Callback<()>>,
) -> impl IntoView {
    let state_label = post.state.to_string();
    let state       = post.state;
    let state_pid   = post.id.to_string();
    // Live tally, bumped optimistically by the +1 button.
    let prayers     = RwSignal::new(post.prayers);


    let post_id        = post.id.to_string();
    let shared_with    = match post.visibility {
        Visibility::Group(gid) => Some(gid),
        _                      => None,
    };

    let (selected, set_selected) = signal(String::new());
    let (msg, set_msg)           = signal(Option::<(bool, String)>::None);

    let share = Action::new(move |(pid, gid): &(String, String)| {
        let pid = pid.clone();
        let gid = gid.clone();
        async move {
            match share_prayer_to_group(pid, gid).await {
                Ok(())  => (true, "Shared to group.".to_string()),
                Err(e)  => (false, e.to_string()),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = share.value().get() {
            set_msg.set(Some(result));
        }
    });

    let share_control = move || {
        if !can_share.get() {
            return None;
        }
        let gs = groups.get();
        if gs.is_empty() {
            return None;
        }
        let post_id = post_id.clone();
        Some(view! {
            <div class="share-row">
                <select
                    class="share-select"
                    on:change=move |e| set_selected.set(event_target_value(&e))
                >
                    <option value="" selected>"Share with a group…"</option>
                    {gs.into_iter().map(|g| {
                        let id = g.id.to_string();
                        let is_current = shared_with.map_or(false, |c| c == g.id);
                        let label = if is_current {
                            format!("{} ✓", g.name)
                        } else {
                            g.name
                        };
                        view! { <option value=id>{label}</option> }
                    }).collect_view()}
                </select>
                <button
                    class="share-btn"
                    disabled=move || share.pending().get()
                    on:click=move |_| {
                        let gid = selected.get();
                        if !gid.is_empty() {
                            set_msg.set(None);
                            share.dispatch((post_id.clone(), gid));
                        }
                    }
                >
                    {move || if share.pending().get() { "Sharing…" } else { "Share" }}
                </button>
            </div>
        })
    };

    view! {
        <article class="post-card">
            <p class="post-kind">{state_label}</p>
            <blockquote class="post-content">{post.content}</blockquote>
            <History
                created_at=post.created_at
                current=post.state
                history=post.history
                prayers=prayers
            />
            {share_control}
            <PostActions
                post_id=state_pid
                state=state
                viewer=viewer
                prayers=prayers
                on_change=on_state_change
            />
            {move || msg.get().map(|(ok, m)| view! {
                <p class=if ok { "invite-ok" } else { "error-msg" }>{m}</p>
            })}
        </article>
    }
}

/// How the owner's UI offers a move from `from` into `to`, or `None` if the
/// state machine doesn't allow it.
struct Offer {
    label: &'static str,
    /// When set, the move opens a panel asking for an optional note about it,
    /// with this placeholder. The note is logged with the move and shown after
    /// the entry; the entry's own text is never rewritten.
    note_prompt: Option<&'static str>,
    /// When set, the panel also shows this warning before the move.
    confirm: Option<String>,
}

/// The `match` on `to` has no wildcard on purpose: a new `PostState` variant
/// won't compile until you decide here how (or whether) the UI offers it.
fn offer(from: PostState, to: PostState) -> Option<Offer> {
    if !from.can_transition_to(to) {
        return None;
    }
    match to {
        // Only a starting state today; nothing moves back into it. If the
        // machine ever allows that, the `every_legal_move_has_an_offer` test fails.
        PostState::Prayer => None,
        PostState::Thanksgiving => Some(Offer {
            label:       "Answered — give thanks",
            note_prompt: Some("How was it answered? (optional)"),
            confirm:     None,
        }),
        PostState::Released => Some(Offer {
            label:       "Release",
            note_prompt: Some("Why are you releasing it? (optional)"),
            confirm:     Some(format!("Release this {}? It will leave your book.", from.as_str())),
        }),
    }
}

/// How a recorded move reads in an entry's history. No wildcard, for the same
/// reason as `offer`.
/// How an entry's first line reads, by the state it was written in. No
/// wildcard, for the same reason as `offer`.
fn created_label(initial: PostState) -> &'static str {
    match initial {
        PostState::Prayer       => "Prayed",
        PostState::Thanksgiving => "Gave thanks",
        // Not a starting state (`is_initial`), so never shown in practice.
        PostState::Released     => "Written",
    }
}

fn history_label(to: PostState) -> &'static str {
    match to {
        PostState::Prayer       => "Returned to prayer",
        PostState::Thanksgiving => "Answered",
        PostState::Released     => "Released",
    }
}

/// The entry's lifecycle log, shown after its content: when it was first
/// written (worded by its starting state, e.g. "Prayed"), then one dated line
/// per move with the note recorded at the time.
///
/// Returns `AnyView` to keep `PostPage`'s view type shallow.
#[component]
fn History(
    created_at: OffsetDateTime,
    /// The entry's state now; its starting state if it has never moved.
    current: PostState,
    history: Vec<PostTransition>,
    /// "Praying now" tally, shown last once anyone has prayed.
    prayers: RwSignal<PrayerCount>,
) -> impl IntoView {
    let date_fmt = format_description::parse("[month repr:long] [day], [year]")
        .unwrap_or_default();
    let initial = history.first().map_or(current, |t| t.from);
    let written = format!(
        "{} · {}",
        created_label(initial),
        created_at.format(&date_fmt).unwrap_or_default(),
    );
    view! {
        <div class="post-history">
            <div class="history-entry">
                <p class="history-meta">{written}</p>
            </div>
            {history.into_iter().map(|t| {
                let when = t.at.format(&date_fmt).unwrap_or_default();
                view! {
                    <div class="history-entry">
                        <p class="history-meta">{format!("{} · {when}", history_label(t.to))}</p>
                        {t.note.map(|n| view! { <p class="history-note">{n}</p> })}
                    </div>
                }
            }).collect_view()}
            {move || {
                let p = prayers.get();
                (p.total > 0).then(|| view! {
                    <div class="history-entry">
                        <p class="history-meta">{prayer_tally(p)}</p>
                    </div>
                })
            }}
        </div>
    }.into_any()
}

/// "12 prayers from 3 people".
fn prayer_tally(p: PrayerCount) -> String {
    let plural = |n: i64, one: &str, many: &str| if n == 1 { one.to_string() } else { many.to_string() };
    format!(
        "{} {} from {} {}",
        p.total, plural(p.total, "prayer", "prayers"),
        p.people, plural(p.people, "person", "people"),
    )
}

/// The control the UI renders for an action.
enum ActionUi {
    /// A lifecycle move, with its button/panel wording.
    Move(PostState, Offer),
    /// The "praying now" +1 button.
    PrayingNow,
}

/// How the UI presents `action` on an entry in state `from`. The `match` has
/// no wildcard on purpose: a new `PostAction` won't compile until you decide
/// here how (or whether) the UI offers it.
fn action_ui(from: PostState, action: PostAction) -> Option<ActionUi> {
    match action {
        PostAction::MoveTo(to) => offer(from, to).map(|o| ActionUi::Move(to, o)),
        PostAction::PrayingNow => Some(ActionUi::PrayingNow),
    }
}

type Move = (String, PostState, Option<String>);

/// One control per action `PostAction::allowed` gives this viewer on this
/// entry: "Praying now" for anyone who can see a prayer, lifecycle moves for
/// the author.
///
/// Returns `AnyView` so its nesting doesn't deepen `PostPage`'s view type
/// (see CLAUDE.md on the release-build depth limit).
#[component]
fn PostActions(
    post_id: String,
    state: PostState,
    viewer: Viewer,
    prayers: RwSignal<PrayerCount>,
    on_change: Option<Callback<()>>,
) -> impl IntoView {
    let (error, set_error) = signal(Option::<String>::None);

    let transition = Action::new(move |(pid, to, note): &Move| {
        let (pid, to, note) = (pid.clone(), *to, note.clone());
        async move {
            match set_post_state(pid, to, note).await {
                Ok(()) => {
                    set_error.set(None);
                    if let Some(cb) = on_change { cb.run(()); }
                }
                Err(e) => set_error.set(Some(e.to_string())),
            }
        }
    });

    let controls = PostAction::all()
        .filter(|a| a.allowed(state, viewer))
        .filter_map(|a| action_ui(state, a))
        .map(|ui| match ui {
            ActionUi::Move(to, o) => view! {
                <OfferButton post_id=post_id.clone() to=to offer=o transition=transition/>
            }.into_any(),
            ActionUi::PrayingNow => view! {
                <PrayButton post_id=post_id.clone() prayers=prayers set_error=set_error/>
            }.into_any(),
        })
        .collect_view();

    view! {
        <div class="state-row">
            {controls}
            {move || error.get().map(|e| view! { <p class="error-msg">{e}</p> })}
        </div>
    }.into_any()
}

/// "Praying now" (+1). Every press counts and is sent immediately, so mashing
/// it works; the tally updates optimistically and settles on the server's.
#[component]
fn PrayButton(
    post_id: String,
    prayers: RwSignal<PrayerCount>,
    set_error: WriteSignal<Option<String>>,
) -> impl IntoView {
    let pray = Action::new(move |pid: &String| {
        let pid = pid.clone();
        async move {
            match pray_for(pid).await {
                Ok(server) => {
                    set_error.set(None);
                    // Responses can land out of order while mashing: only move
                    // forward, never back to an older total.
                    prayers.update(|p| {
                        p.total  = p.total.max(server.total);
                        p.people = p.people.max(server.people);
                    });
                }
                Err(e) => {
                    prayers.update(|p| p.total = (p.total - 1).max(0));
                    set_error.set(Some(e.to_string()));
                }
            }
        }
    });

    view! {
        <button
            class="state-btn pray-btn"
            title="Let them know you're praying"
            on:click=move |_| {
                prayers.update(|p| p.total += 1);
                pray.dispatch(post_id.clone());
            }
        >"🙏 Praying now"</button>
    }.into_any()
}

/// One transition button. If the offer wants a note or a confirmation, the
/// button opens an inline panel for them first.
#[component]
fn OfferButton(
    post_id: String,
    to: PostState,
    offer: Offer,
    transition: Action<Move, ()>,
) -> impl IntoView {
    let Offer { label, note_prompt, confirm } = offer;
    let pending = move || transition.pending().get();

    if note_prompt.is_none() && confirm.is_none() {
        return view! {
            <button
                class="state-btn"
                disabled=pending
                on:click=move |_| { transition.dispatch((post_id.clone(), to, None)); }
            >{label}</button>
        }.into_any();
    }

    let (open, set_open) = signal(false);
    let (note, set_note) = signal(String::new());
    // Hold the book on this page while the panel is open, so a key press or
    // swipe can't turn the page and discard the note being written.
    use_folio_lock(open);
    view! {
        <Show
            when=move || open.get()
            fallback=move || view! {
                <button class="state-btn" on:click=move |_| set_open.set(true)>{label}</button>
            }
        >
            <div class="state-panel">
                {confirm.clone().map(|c| view! { <p class="state-confirm">{c}</p> })}
                {note_prompt.map(|placeholder| view! {
                    <textarea
                        class="state-note"
                        rows="3"
                        placeholder=placeholder
                        prop:value=note
                        on:input=move |e| set_note.set(event_target_value(&e))
                    />
                })}
                <div class="state-panel-actions">
                    <button
                        class="state-btn"
                        disabled=pending
                        on:click={
                            let pid = post_id.clone();
                            move |_| {
                                let n = note.get_untracked();
                                let n = (!n.trim().is_empty()).then_some(n);
                                transition.dispatch((pid.clone(), to, n));
                            }
                        }
                    >{label}</button>
                    <button class="state-btn" on:click=move |_| set_open.set(false)>"Cancel"</button>
                </div>
            </div>
        </Show>
    }.into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_allowed_action_has_a_control() {
        for state in PostState::ALL {
            for viewer in [Viewer::Author, Viewer::Other] {
                for action in PostAction::all() {
                    if action.allowed(state, viewer) {
                        assert!(
                            action_ui(state, action).is_some(),
                            "{action:?} is allowed on {state:?} for {viewer:?} but the UI has no control",
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_legal_move_has_an_offer() {
        for from in PostState::ALL {
            for to in PostState::ALL {
                assert_eq!(
                    from.can_transition_to(to),
                    offer(from, to).is_some(),
                    "{from:?} → {to:?}: the state machine and the UI disagree",
                );
            }
        }
    }
}
