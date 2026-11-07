---
title: "Rewriting this site in ratzilla"
date: "2026-11-07"
description: "The site is a TUI now, built with ratzilla."
---

I always wanted my site to be a TUI. What kept me from doing it were embeds: when you paste a link in Discord or wherever, you want a proper preview, and for that the OpenGraph tags have to be in the HTML before any JS runs. The site was in Leptos at the time, and getting OG tags per blog post working there was annoying enough that I just moved to SvelteKit.

Now everyone has a site Claude built for them, so I finally did it too. Claude Opus 5.5 rewrote the whole thing with [ratzilla](https://github.com/ratatui/ratzilla), which is ratatui running in the browser.

## Keys

Vim bindings. `v` to pick a link.
