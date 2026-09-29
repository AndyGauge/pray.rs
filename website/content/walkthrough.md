---
title: "How pray.rs works"
lede: "A walk through everything you can do: writing, sharing, praying for one another, and watching prayers become thanksgivings."
description: "A walkthrough of pray.rs: writing prayers, the book, groups and QR codes, praying now, answered prayers, releasing, and bringing your own AI."
---

## Write a prayer, or a thanksgiving

{{< illustration "write" "Someone sits at a table writing in an open prayer book by candlelight." >}}

Tap **+ New**, choose **Prayer** or **Thanksgiving**, and write it in your own words.
Then choose who can see it:

- **Private**: only you. This is where most prayers live.
- **A group**: the people you pray with.
- **Public**: anyone on pray.rs, and the [public prayers]({{< relref "prayers" >}}) page here.

## A book, not a feed

{{< illustration "book" "Someone swipes on their phone and a single page of the prayer book turns over." >}}

pray.rs shows one page at a time. Swipe, use the arrow keys, or tap **← →** to turn the
page. There is no infinite scroll and nothing is ranked. Choose whose pages you're
reading: **Mine**, **Public**, or one of your groups.

## Start a group in real space

{{< illustration "qr-join" "Two people stand together outside. One holds up a phone showing the group's QR code; the other scans it with their own phone to join." >}}

Groups work best with people you actually pray with. Create a group, open it, and show
its **QR code**. Anyone who scans it with their phone joins, right there. You can also
copy the join link, or make a new one if the old link has travelled too far.

## Or invite by email or text

{{< illustration "invite" "Someone waves, and an invitation envelope flies across to a friend in another home, who opens it on their phone." >}}

From a group, enter someone's email address or phone number to send an invitation.
They follow the link, sign in, and they're in.

## Share a prayer with your group

{{< illustration "share" "Someone hands a written prayer toward three friends gathered together, sharing it with the group." >}}

Under any of your own prayers, pick a group and tap **Share**. It moves into that
group's pages, where the others can read it and pray for it.

## Praying now

{{< illustration "praying-now" "Two people in different places. One taps the praying-now button on their phone, and the other sees the count of prayers for their request go up." >}}

When you pray for someone's prayer, tap **🙏 Praying now**. Tap it every time you pray;
every tap counts. The prayer shows its running tally ("12 prayers from 3 people") so
the person who asked knows they're being carried. Anyone who can see a prayer can pray
for it: its author, their group, or everyone, if it's public.

## When a prayer is answered

{{< illustration "answered" "Someone raises both hands in thanks in the sunshine. In the book, the original prayer stays as written, with a note about how it was answered added underneath." >}}

Tap **Answered: give thanks** and, if you like, write how it was answered. The prayer
becomes a thanksgiving. Your original words are never changed: the answer is recorded
after them, with its date, so the page reads as the whole story:

> *Prayed · June 3* … *Answered · September 27*: "Made it through."

## Release a prayer

{{< illustration "release" "Someone opens their hands and a bird flies up and away: a prayer being released." >}}

Some prayers are ready to be let go. **Release** takes a prayer or thanksgiving out of
your book (and out of every group or public page), with an optional note about why.

## Bring your own AI

{{< illustration "ai" "Someone at a laptop talks with their AI assistant, which connects to their prayer book through MCP and acts only as them." >}}

pray.rs has a built-in [MCP](https://modelcontextprotocol.io) server, so an AI assistant
you already use (Claude, and other MCP clients) can read and write your prayer book, signed in
as you and limited to what you allow. In Claude Code:

```sh
claude mcp add --transport http prayers https://pray.rs/mcp
```

Then run `/mcp`, choose **prayers**, and **Authenticate**. Your assistant can list and
write prayers, give thanks with a note, release, share with a group, and tap **praying
now** for you. This site uses the same server, signed out, to collect the
[public prayers]({{< relref "prayers" >}}).

## Your data stays yours

- **Log out** (Settings) signs you out and clears this device's copy of the app.
- **Delete my data** removes your account and everything you've written.
- The [privacy policy](https://pray.rs/privacy) says what's kept and why.
