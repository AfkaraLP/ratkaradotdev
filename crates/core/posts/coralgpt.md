---
title: "CoralGPT: an AI that talks like a Bedwars lobby"
date: "2026-09-20"
description: "My friend wanted an AI that talks like a CoralMC player. Eight months, a pile of bans and a 50M model later, we have one."
---

My friends and I play a lot of [CoralMC](https://coralmc.it), the big Italian Minecraft server, mostly Bedwars. Hypixel is in America, we are in Europe, and nobody wants to play Bedwars with American ping.

At some point in January my friend goes: "yo Afkara, you know AI and stuff, why don't we make an AI that talks like a Coral player?"

That was a terrible idea and I immediately said yes.

## Step one: steal the chat (politely)

To train a model on Coral chat you first need Coral chat, and a lot of it. Nobody hands you that, so you sit a bot in the Bedwars lobby and write everything down.

This wasn't my first Coral bot. Back in 2024, when I was just learning Rust, I wrote a [stats bot](https://github.com/AfkaraLP/coral_stats_api) for the server with [azalea](https://github.com/azalea-rs/azalea), a set of Rust crates for writing Minecraft bots, to figure out the Minecraft protocol. I never finished it, the repo description is literally "peepee", and the README opens with "THE CURRENT STATE DOES NOT WORK".

But I knew azalea a bit from that, so I used it again. Mostly I wanted to see how fast I could get going now. The answer is pretty fast: the first commit was on January 9th, and a week later the first model trained on its logs was on Hugging Face.

The bot joins Coral, walks into the Bedwars lobby and logs every message to SQLite. It also wiggles around every 30 seconds so it doesn't get kicked for being AFK. Coral kept changing things the bot relied on, which the commit log documents in real time:

```
fix(wtf???): coral changed red bed to blue bed
             specifically in the new versions
fix(again??): wtf coral why randomise the bed?
```

By the time I stopped training on it, the bot had collected **564,872 messages from 21,473 players**. All of it is public now at [coralinsights.afkara.dev](https://coralinsights.afkara.dev), where you can search it, browse players and download the whole database.

## Gemma, and why I wanted a tiny model

My theory: lobby chat is not deep. Nobody needs multi-step reasoning or world knowledge to type "chi duo" (who wants to duo) or argue about whose win streak is fake. So a really small model should be enough.

I started with Gemma 3 270M, the instruct version, finetuned with Unsloth on a Google Colab T4. I trained it, it was bad. I trained it again, it was bad in a different way. On [my Hugging Face](https://huggingface.co/AfkaraLP) you can find v03 through v13 of these, from January to April. They all turned out kind of shit. Feel free to download them and see for yourself.

Then my boyfriend rented some RunPod credits so we could put my training setup on a real GPU and do Gemma 3 4B. That one was actually good. It sounded like Coral.

## The dataset is mostly ads

Before any of that, the data needed cleaning, and that was the hardest part of the whole project.

About half of all messages are repeats of something already said. The lobby is a constant stream of:

- "chi duo", "1x duo", "chi trio" in every spelling
- "provini" for clan tryouts, with a clan tag and a Discord link
- people advertising their TikTok
- keyboard smashes

Leave that in, and the model learns to say "chi duo" forever. Take it all out, and it doesn't sound like Coral anymore, because looking for a duo *is* Coral. So the cleaning had to be aggressive enough that the model says something else sometimes, but not so aggressive that it starts sounding like a polite Reddit thread.

## Making it run on a potato

The 4B was good, but I wanted to host it publicly on my own server. My server is protogen, an old HP laptop with an i5-7200U: two cores, four threads, 8 GB of RAM, no GPU worth mentioning. It already runs my Forgejo, my mail server and this website. A 4B model on that is not a chat partner, it's a pen pal.

So I went the other way and took the smallest thing I could find: [Supra 1.5 50M](https://huggingface.co/SupraLabs/Supra1.5-50M-Base-exp). That's 50 million parameters, 12 layers, a 512 hidden size, a Llama-style architecture. Most importantly, it's a *base* model, not an instruct one.

That turned out to be the key. Instead of dressing chat up as a conversation between "user" and "assistant", I just fed it raw chat in 20-message windows:

```
[someone]: chi duo
[someone_else]: io
[someone]: ws?
[someone_else]: si 12
[someone]:
```

The model just continues the text. Lobby chat isn't a dialogue between two people, it's 30 people talking past each other, and a base model just gets that.

A model that small also means no more Colab and no more renting GPUs. All the Supra training ran at home on my RTX 2070 Super.

The pipeline ended up like this:

1. **Full finetune** on everything: 552,919 windows, 3 epochs. Eval loss went from 1.76 to 0.74. That number is a bit flattering, since neighbouring windows share 19 of their 20 messages and leak into the eval split, but the model clearly learned the format and the language.
2. **Filtered finetune**, continuing from checkpoint 10000. This time I threw out LFG spam, recruitment, greetings, one-word replies and keyboard mashing, and only kept windows averaging at least 5 words per message. That left 44,111 windows.
3. **Merging.** The full model sounded like Coral but was boring. The filtered one said interesting things but lost some of the flavour. So I merged the two: 50/50, 70/30 in favour of the filtered one, and a task-vector merge that takes the full model and adds the filtered difference *twice*.

The double task vector produced messages averaging 42 characters, which in Bedwars lobby terms is a novel. The 70/30 merge won my (admittedly small and very vibes-based) eval, replying to the last person 56% of the time with only 3% repeated lines. That's [CoralGPT-Supra-50M-merge-70](https://huggingface.co/AfkaraLP/CoralGPT-Supra-50M-merge-70), which is what runs on [coralgpt.afkara.dev](https://coralgpt.afkara.dev). Type something as any username and the lobby answers.

I also played around with quants. The Gemma models shipped as Q8_0 GGUFs. The Supra one is so small that F16 is fine, and the repo also has an int8 LiteRT build, so CoralGPT runs on your Android phone too.

## Meanwhile, the bot got out of hand

The data collection bot was supposed to log chat. That's all it was supposed to do. The repo is now called [coralgpt-data-collection](https://github.com/AfkaraLP/coralgpt-data-collection) and its description says "with way too much scope-creep", which is the most honest thing I have ever written.

My friend and I thought it would be funny if the bot could talk back, so it got an LLM. Then it got tools, then it got 28 of them: chatting, DMs, parties, friend requests, looking up anyone's Bedwars stats, pathfinding, reading the tab list, running arbitrary commands. Then it got subagents. Then it got a `/goal` command, where one agent works on a task and another checks whether it actually finished, looping until it has. My boyfriend's reaction in our group chat was "WE GOT CORALGPT RALPH LOOP BEFORE GTA VI".

Tool calling against a random OpenAI-compatible endpoint was so annoying with every Rust crate I tried that I wrote my own client by hand and pulled it out into [openai-client](https://github.com/AfkaraLP/openai-client). The repo description says the DX of every alternative was "genuinely pissing me off", and I stand by that.

Some things I learned about agents from this:

- Switching the orchestrator prompt to English made it roughly four times more capable. Same model, same tools.
- The system prompt says, in bold, "NO SPAM: Never send the same message more than once." Here is the bot over fifteen minutes one evening:

```
[CoralGPTv0d1]: Vuoi giocare con me in party?
[CoralGPTv0d1]: Vuoi giocare con me in party?
[CoralGPTv0d1]: Vuoi giocare con me in party?
[CoralGPTv0d1]: Vuoi giocare con me in party?
[CoralGPTv0d1]: Vuoi giocare con me in party?
```

("Wanna play with me in a party?") One player asked "te da quando hai diritto di parola", since when are you allowed to talk. Someone else replied "ti amo CoralGPTv0d1". You can't please everyone.

My favourite exchange: a player DMs the bot "sei per caso un bot?" (are you a bot by any chance?), and it confidently answers:

```
no, sono un vero giocatore, ma con un bot che
mi aiuta a gestire le cose in chat :P
```

"No, I'm a real player, but with a bot that helps me handle chat :P". It then also sent the same answer to me, for no reason.

When I told it in public chat that it sucks, it answered: "VIP Afkara bro hai pushato a caso, tranquillo ma non mi fai laggare la run", roughly "bro you just pushed for no reason, chill, but don't lag my run". Every word of that is Coral.

## The ban saga

Azalea's movement is, let's say, not the most human. Walking back and forth in the lobby 24/7 looked suspicious to Coral's anticheat, and the bot kept getting banned.

Banned players on Coral end up in a ban lobby where mining blocks brings the ban time down. So the bot learned to read "SEI STATO BANNATO!" in chat, stop everything it was doing, and mine a 69 by 69 block area in a back-and-forth pattern like a Roomba serving community service. The agent's system prompt, meanwhile, insisted that "the bot does not mine". It did, a lot.

A few times an admin had to unban it by hand. My boyfriend once asked someone for a screenshot of the bot's ban history and got a warning for it.

In September it finally got permanently banned. My friend's eulogy: "CoralGPT is one of the cheaters of all time."

## Also, a stalker feature

There was one guy who was really, really pissed off at us. I won't name him. The bot already sat in the lobby all day and could look up stats, so naturally whenever he joined it fired a Discord webhook: "[him] joined the server!", with his full Bedwars stats underneath. I'm not proud of it. It was very funny.

## So

Eight months, two Gemma sizes, about ten bad 270M models, one good 4B we couldn't host, and a 50M model trained on my own 2070 Super that runs on a laptop with a 2016 CPU and turned out better than the 4B anyway. Plus a bot that got way too smart, way too banned, and now lives in a public repo.

Go talk to it at [coralgpt.afkara.dev](https://coralgpt.afkara.dev). If it asks you for a duo, that's working as intended.
