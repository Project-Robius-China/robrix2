# Robrix Design Document

## Overview

Robrix is a multi-platform Matrix chat client written in pure Rust using the Makepad UI framework and Project Robius application development framework. It targets macOS, Windows, Linux, Android, iOS, and OpenHarmony.

## Architecture

### Three-Layer Architecture

```
+─────────────────────────────────────────────────────+
│                   UI Layer                           │
│  Makepad script_mod! DSL, Widgets, MatchEvent       │
│  (app.rs, home/, shared/, room/, login/, settings/)  │
+─────────────────────────────────────────────────────+
                         │
                   Actions / Scope
                         │
+─────────────────────────────────────────────────────+
│                Matrix Protocol Layer                 │
│  sliding_sync.rs — async client, auth, timelines     │
│  space_service_sync.rs — space hierarchy             │
│  submit_async_request() → tokio background tasks     │
+─────────────────────────────────────────────────────+
                         │
                Cx::post_action / MPSC
                         │
+─────────────────────────────────────────────────────+
│              Persistence & Cache Layer               │
│  persistence/ — session, app state, window geometry  │
│  avatar_cache, media_cache, user_profile_cache       │
│  account_manager — multi-account switching           │
+─────────────────────────────────────────────────────+
```

### Key Components

| Component | File(s) | Responsibility |
|-----------|---------|----------------|
| App | `app.rs` | Root state, event dispatch, modal management |
| Sliding Sync | `sliding_sync.rs` | Matrix client lifecycle, room sync, timeline subscriptions |
| Room Screen | `home/room_screen.rs` | Timeline rendering, message display, pagination |
| Rooms List | `home/rooms_list.rs` | Room list with categories (invited, direct, regular) |
| Room Input Bar | `room/room_input_bar.rs` | Message composition, replies, mentions |
| Mentionable Text Input | `shared/mentionable_text_input.rs` | @mention autocomplete with background search |
| Command Text Input | `shared/command_text_input.rs` | Generic popup/autocomplete infrastructure |
| HTML/Plaintext | `shared/html_or_plaintext.rs` | Message rendering with Matrix HTML support |

### Technology Stack

- **UI Framework**: Makepad 2.0 (`script_mod!` DSL, `Script`/`ScriptHook` derives)
- **Matrix SDK**: `matrix-sdk` with sliding sync, E2E encryption, SQLite storage
- **Async Runtime**: Tokio
- **Serialization**: Serde (JSON for persistence, RON for legacy)

### UI Patterns (Makepad 2.0)

- **Widget DSL**: `script_mod!` blocks define widget trees with Splash syntax
- **Named children**: Use `:=` operator (NOT `=`) for addressable widgets
- **Property merge**: Use `+:` to extend inherited properties, `:` to replace
- **Event flow**: `handle_event` → `MatchEvent::handle_actions` → widget action queries
- **State changes on dynamic widgets**: Use Animator + shader instance variables (NOT `script_apply_eval!` which fails on `widget_ref_from_live_ptr()` widgets due to `ScriptObject::ZERO`)
- **Runtime property limits**: `script_apply_eval!` cannot use DSL constants (`Right`, `Fit`, `Align`) — bake into templates or use `#(rust_expr)` interpolation

### Async Communication Pattern

```
UI Thread                          Background Thread
    │                                     │
    ├── submit_async_request() ──────────►│ MatrixRequest::*
    │                                     │
    │◄── Cx::post_action() ──────────────┤ Result action
    │                                     │
    ├── handle_actions() processes result  │
```

### Persistence

- Session data: `~/.local/share/org.robius.robrix/<user>/persistent_state/`
- App state: `latest_app_state.json` (dock layout, open rooms, selected room)
- Window geometry: `window_geom_state.json`

## Module Organization

```
src/
├── app.rs                  # Root app, modals, global state
├── sliding_sync.rs         # Matrix client, sync, requests
├── space_service_sync.rs   # Space hierarchy
├── cpu_worker.rs           # Background CPU tasks
├── home/                   # Main UI screens
│   ├── room_screen.rs      # Timeline + message display
│   ├── rooms_list.rs       # Room list sidebar
│   ├── main_desktop_ui.rs  # Desktop dock layout
│   ├── home_screen.rs      # Adaptive desktop/mobile
│   └── ...
├── shared/                 # Reusable widgets
│   ├── mentionable_text_input.rs  # @mention system
│   ├── command_text_input.rs      # Popup autocomplete
│   ├── html_or_plaintext.rs       # Message rendering
│   ├── avatar.rs                  # Avatar display
│   └── ...
├── room/                   # Room-specific logic
│   ├── room_input_bar.rs   # Message input
│   ├── member_search.rs    # Member search algorithm
│   └── ...
├── login/                  # Authentication
├── logout/                 # Session cleanup
├── settings/               # User preferences
├── persistence/            # State storage
├── profile/                # User profiles
├── i18n.rs                 # Internationalization
└── utils.rs                # Shared utilities
```
