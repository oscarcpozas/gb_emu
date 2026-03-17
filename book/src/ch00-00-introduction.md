# Introduction

> [!NOTE]
> First things first. If you've somehow ended up here, this is the documentation/notes for my
> [Github project to emulate the Gameboy in Rust](https://github.com/oscarcpozas/gb_emu_rs).

I think a good way to understand how a computer and its components work from a
software/coding perspective is by trying to emulate their behaviors and relationships. That's why I chose one of the
most iconic portable consoles ever - Nintendo's Gameboy. The idea is to replicate this console's behavior 100%
while understanding how this type of system works.

I'm choosing the Gameboy because there's already tons of documentation and other projects that have accomplished this same goal
that I can lean on and try not to drag the process out too long.

## Why Rust?

It's pretty normal for projects that try to emulate systems to use languages considered low-level
since performance is a key factor. However, it's true that the Gameboy is so undemanding for
modern computers that we can solve this problem in pretty much any language.

**So why Rust?** It was a low-level language I wanted to try doing something challenging with.
Plus, what I learn from this project could be useful for future emulators I might want to make where I
actually need to worry more about performance. It's also a language with a solid community I can lean on
along the way.
