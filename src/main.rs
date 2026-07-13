use std::collections::BTreeMap;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;
use zellij_tile::prelude::*;

// ========== COLOR SYSTEM ==========

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum ColorSpec {
    #[default]
    Default,
    EightBit(u8),
    Rgb(u8, u8, u8),
}

impl ColorSpec {
    fn to_ansi_fg(self) -> String {
        match self {
            ColorSpec::Default => String::new(),
            ColorSpec::EightBit(n) => format!("\x1b[38;5;{}m", n),
            ColorSpec::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
        }
    }

    fn to_ansi_bg(self) -> String {
        match self {
            ColorSpec::Default => String::new(),
            ColorSpec::EightBit(n) => format!("\x1b[48;5;{}m", n),
            ColorSpec::Rgb(r, g, b) => format!("\x1b[48;2;{};{};{}m", r, g, b),
        }
    }

    fn is_default(self) -> bool {
        matches!(self, ColorSpec::Default)
    }
}

fn parse_color_spec(name: &str) -> ColorSpec {
    let name = name.trim();

    if let Some(hex) = name.strip_prefix('#')
        && let Some((r, g, b)) = parse_hex_color(hex)
    {
        return ColorSpec::Rgb(r, g, b);
    }

    if let Some(inner) = name.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')'))
        && let Some((r, g, b)) = parse_rgb_func(inner)
    {
        return ColorSpec::Rgb(r, g, b);
    }

    if let Ok(n) = name.parse::<u8>() {
        return ColorSpec::EightBit(n);
    }

    match name.to_lowercase().as_str() {
        "none" | "default" | "reset" => ColorSpec::Default,
        "accent" | "primary" => ColorSpec::EightBit(39),
        "secondary" => ColorSpec::EightBit(75),
        "tertiary" => ColorSpec::EightBit(141),
        "muted" | "quaternary" => ColorSpec::EightBit(245),
        "dim" | "dimmed" => ColorSpec::EightBit(240),
        "black" => ColorSpec::EightBit(0),
        "red" | "error" | "warning" => ColorSpec::EightBit(196),
        "green" | "success" | "ok" => ColorSpec::EightBit(82),
        "yellow" => ColorSpec::EightBit(226),
        "blue" => ColorSpec::EightBit(33),
        "magenta" => ColorSpec::EightBit(201),
        "cyan" => ColorSpec::EightBit(51),
        "white" => ColorSpec::EightBit(15),
        "orange" => ColorSpec::EightBit(208),
        "gray" | "grey" => ColorSpec::EightBit(244),
        "pink" => ColorSpec::EightBit(213),
        "purple" => ColorSpec::EightBit(135),
        _ => ColorSpec::Default,
    }
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    match hex.len() {
        3 => {
            let mut chars = hex.chars();
            let r = chars.next()?.to_digit(16)? as u8 * 17;
            let g = chars.next()?.to_digit(16)? as u8 * 17;
            let b = chars.next()?.to_digit(16)? as u8 * 17;
            Some((r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

fn sanitize_terminal_text(s: &str) -> String {
    s.chars()
        .map(|ch| if ch.is_control() { '?' } else { ch })
        .collect()
}

fn parse_rgb_func(inner: &str) -> Option<(u8, u8, u8)> {
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].trim().parse::<u8>().ok()?;
    let g = parts[1].trim().parse::<u8>().ok()?;
    let b = parts[2].trim().parse::<u8>().ok()?;
    Some((r, g, b))
}

// ========== STYLE SYSTEM ==========

#[derive(Debug, Clone, Default)]
struct InlineStyle {
    fg: ColorSpec,
    bg: ColorSpec,
    bold: bool,
    dim: bool,
    fill: bool,
}

impl InlineStyle {
    fn to_ansi(&self) -> String {
        let mut result = String::new();
        if self.bold {
            result.push_str("\x1b[1m");
        }
        if self.dim {
            result.push_str("\x1b[2m");
        }
        result.push_str(&self.fg.to_ansi_fg());
        result.push_str(&self.bg.to_ansi_bg());
        result
    }

    fn has_any_style(&self) -> bool {
        !self.fg.is_default() || !self.bg.is_default() || self.bold || self.dim || self.fill
    }
}

#[derive(Debug, Clone)]
struct StyledSegment {
    text: String,
    style: InlineStyle,
}

impl StyledSegment {
    fn display_width(&self) -> usize {
        self.text.width()
    }
}

#[derive(Debug, Clone, Default)]
struct StyledText {
    segments: Vec<StyledSegment>,
}

impl StyledText {
    fn new() -> Self {
        Self { segments: vec![] }
    }

    fn push(&mut self, text: String, style: InlineStyle) {
        if !text.is_empty() {
            self.segments.push(StyledSegment { text, style });
        }
    }

    fn display_width(&self) -> usize {
        self.segments.iter().map(|s| s.display_width()).sum()
    }

    fn to_ansi(&self) -> String {
        let mut result = String::new();
        for segment in &self.segments {
            if segment.style.has_any_style() {
                result.push_str("\x1b[0m");
                result.push_str(&segment.style.to_ansi());
            }
            result.push_str(&segment.text);
        }
        if self.segments.iter().any(|s| s.style.has_any_style()) {
            result.push_str("\x1b[0m");
        }
        result
    }

    fn truncate(&self, max_width: usize) -> StyledText {
        if self.display_width() <= max_width {
            return self.clone();
        }
        let mut result = StyledText::new();
        let mut remaining = max_width;
        for segment in &self.segments {
            if remaining == 0 {
                break;
            }
            let seg_width = segment.display_width();
            if seg_width <= remaining {
                result.push(segment.text.clone(), segment.style.clone());
                remaining -= seg_width;
            } else {
                let mut truncated = String::new();
                let mut width = 0;
                for ch in segment.text.chars() {
                    let ch_width = ch.to_string().width();
                    if width + ch_width > remaining {
                        break;
                    }
                    truncated.push(ch);
                    width += ch_width;
                }
                result.push(truncated, segment.style.clone());
                break;
            }
        }
        result
    }
}

// ========== FORMAT PARSING ==========

#[derive(Debug, Clone)]
enum FormatToken {
    Style(InlineStyle),
    Variable { name: String, width: Option<usize> },
    Literal(String),
}

fn parse_tmux_format(format: &str) -> Vec<FormatToken> {
    let mut tokens = Vec::new();
    let mut chars = format.chars().peekable();
    let mut literal = String::new();

    while let Some(ch) = chars.next() {
        if ch == '#' {
            match chars.peek() {
                Some('[') => {
                    if !literal.is_empty() {
                        tokens.push(FormatToken::Literal(std::mem::take(&mut literal)));
                    }
                    chars.next();
                    let mut style_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == ']' {
                            chars.next();
                            break;
                        }
                        style_str.push(chars.next().unwrap());
                    }
                    tokens.push(FormatToken::Style(parse_style_directive(&style_str)));
                }
                Some('{') => {
                    if !literal.is_empty() {
                        tokens.push(FormatToken::Literal(std::mem::take(&mut literal)));
                    }
                    chars.next();
                    let var_token = parse_variable(&mut chars);
                    tokens.push(var_token);
                }
                _ => {
                    literal.push(ch);
                }
            }
        } else if ch == '{' {
            if !literal.is_empty() {
                tokens.push(FormatToken::Literal(std::mem::take(&mut literal)));
            }
            let var_token = parse_variable(&mut chars);
            tokens.push(var_token);
        } else {
            literal.push(ch);
        }
    }

    if !literal.is_empty() {
        tokens.push(FormatToken::Literal(literal));
    }

    tokens
}

fn parse_style_directive(content: &str) -> InlineStyle {
    let mut style = InlineStyle::default();
    for part in content.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(color_str) = part.strip_prefix("fg=") {
            style.fg = parse_color_spec(color_str);
        } else if let Some(color_str) = part.strip_prefix("bg=") {
            style.bg = parse_color_spec(color_str);
        } else if part == "bold" {
            style.bold = true;
        } else if part == "dim" {
            style.dim = true;
        } else if part == "fill" {
            style.fill = true;
        } else if part == "default" || part == "none" || part == "reset" {
            style = InlineStyle::default();
        }
    }
    style
}

fn parse_variable(chars: &mut std::iter::Peekable<std::str::Chars>) -> FormatToken {
    let mut content = String::new();
    while let Some(&c) = chars.peek() {
        if c == '}' {
            chars.next();
            break;
        }
        content.push(chars.next().unwrap());
    }
    if let Some(rest) = content.strip_prefix('=')
        && let Some(colon_pos) = rest.find(':')
    {
        let width_str = &rest[..colon_pos];
        let var_name = &rest[colon_pos + 1..];
        if let Ok(width) = width_str.parse::<usize>() {
            return FormatToken::Variable {
                name: var_name.to_string(),
                width: Some(width),
            };
        }
    }
    FormatToken::Variable {
        name: content,
        width: None,
    }
}

fn parse_styled_string(s: &str) -> StyledText {
    let tokens = parse_tmux_format(s);
    let mut result = StyledText::new();
    let mut current_style = InlineStyle::default();
    for token in tokens {
        match token {
            FormatToken::Style(style) => {
                current_style = style;
            }
            FormatToken::Literal(text) => {
                result.push(text, current_style.clone());
            }
            FormatToken::Variable { name, .. } => {
                result.push(format!("{{{}}}", name), current_style.clone());
            }
        }
    }
    result
}

// ========== CONFIGURATION ==========

#[derive(Clone)]
struct StyleConfig {
    format: String,
    format_active: String,
    format_selected: String,
    format_resurrectable: String,
    overflow_above: String,
    overflow_below: String,
    indicator_active: String,
    indicator_clients: String,
    padding_top: usize,
    border: String,
    max_name_length: usize,
    start_index: usize,
    show_resurrectable: bool,
    section_live: String,
    section_resurrectable: String,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            format: "{index}:{name}".to_string(),
            format_active: "{index}:{name} {indicators}".to_string(),
            format_selected: "#[fg=accent,bold]> {index}:{name} {indicators}".to_string(),
            format_resurrectable: "{index}:{name} (dead)".to_string(),
            overflow_above: "  ^ +{count}".to_string(),
            overflow_below: "  v +{count}".to_string(),
            indicator_active: "*".to_string(),
            indicator_clients: "{n}c".to_string(),
            max_name_length: 20,
            padding_top: 0,
            border: String::new(),
            start_index: 1,
            show_resurrectable: true,
            section_live: String::new(),
            section_resurrectable: String::new(),
        }
    }
}

// ========== SESSION ENTRY ==========

/// Unified entry for both live and resurrectable sessions
#[derive(Clone)]
enum SessionEntry {
    Live(SessionInfo),
    Resurrectable(String, #[allow(dead_code)] Duration),
}

impl SessionEntry {
    fn name(&self) -> &str {
        match self {
            SessionEntry::Live(info) => &info.name,
            SessionEntry::Resurrectable(name, _) => name,
        }
    }

    fn is_current(&self) -> bool {
        match self {
            SessionEntry::Live(info) => info.is_current_session,
            SessionEntry::Resurrectable(_, _) => false,
        }
    }

    fn is_resurrectable(&self) -> bool {
        matches!(self, SessionEntry::Resurrectable(_, _))
    }
}

// ========== PLUGIN STATE ==========

#[derive(Default)]
struct State {
    sessions: Vec<SessionEntry>,
    active_session_idx: usize,
    selected_session_idx: Option<usize>,
    mode_info: ModeInfo,
    style: StyleConfig,
    last_rows: usize,
    permissions_granted: bool,
    is_selectable: bool,
    pending_events: Vec<Event>,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        if let Some(v) = configuration.get("format") {
            self.style.format = v.clone();
        }
        if let Some(v) = configuration.get("format_active") {
            self.style.format_active = v.clone();
        }
        if let Some(v) = configuration.get("format_selected") {
            self.style.format_selected = v.clone();
        }
        if let Some(v) = configuration.get("format_resurrectable") {
            self.style.format_resurrectable = v.clone();
        }
        if let Some(v) = configuration.get("overflow_above") {
            self.style.overflow_above = v.clone();
        }
        if let Some(v) = configuration.get("overflow_below") {
            self.style.overflow_below = v.clone();
        }
        if let Some(v) = configuration.get("indicator_active") {
            self.style.indicator_active = v.clone();
        }
        if let Some(v) = configuration.get("indicator_clients") {
            self.style.indicator_clients = v.clone();
        }
        if let Some(v) = configuration.get("max_name_length")
            && let Ok(n) = v.parse::<usize>()
        {
            self.style.max_name_length = n;
        }
        if let Some(v) = configuration.get("padding_top")
            && let Ok(n) = v.parse::<usize>()
        {
            self.style.padding_top = n;
        }
        if let Some(v) = configuration.get("border") {
            self.style.border = v.clone();
        } else if let Some(v) = configuration.get("border_char") {
            self.style.border = v.clone();
        }
        if let Some(v) = configuration.get("start_index")
            && let Ok(n) = v.parse::<usize>()
        {
            self.style.start_index = n;
        }
        if let Some(v) = configuration.get("show_resurrectable") {
            self.style.show_resurrectable = v == "true";
        }
        if let Some(v) = configuration.get("section_live") {
            self.style.section_live = v.clone();
        }
        if let Some(v) = configuration.get("section_resurrectable") {
            self.style.section_resurrectable = v.clone();
        }

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        subscribe(&[
            EventType::SessionUpdate,
            EventType::ModeUpdate,
            EventType::Mouse,
            EventType::PermissionRequestResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;

        if let Event::PermissionRequestResult(status) = event {
            if status == PermissionStatus::Granted {
                self.permissions_granted = true;
                self.is_selectable = false;
                set_selectable(false);

                // Fetch initial session list proactively (we won't get a
                // SessionUpdate event unless a session change occurs)
                if let Ok(snapshot) = get_session_list() {
                    let mut entries: Vec<SessionEntry> = snapshot
                        .live_sessions
                        .into_iter()
                        .map(SessionEntry::Live)
                        .collect();

                    if self.style.show_resurrectable {
                        for (name, duration) in snapshot.resurrectable_sessions {
                            entries.push(SessionEntry::Resurrectable(name, duration));
                        }
                    }

                    let active_idx = entries.iter().position(|e| e.is_current()).unwrap_or(0);

                    self.active_session_idx = active_idx;
                    self.sessions = entries;
                }

                while !self.pending_events.is_empty() {
                    let cached_event = self.pending_events.remove(0);
                    self.update(cached_event);
                }
                should_render = true;
            }
            return should_render;
        }

        if !self.permissions_granted {
            self.pending_events.push(event);
            return false;
        }

        match event {
            Event::PermissionRequestResult(_) => {}
            Event::ModeUpdate(mode_info) => {
                let was_in_session_mode = self.mode_info.mode == InputMode::Session;
                let is_in_session_mode = mode_info.mode == InputMode::Session;

                if !was_in_session_mode && is_in_session_mode {
                    self.selected_session_idx = Some(self.active_session_idx);
                } else if was_in_session_mode && !is_in_session_mode {
                    self.selected_session_idx = None;
                }

                if self.mode_info != mode_info {
                    should_render = true;
                }
                self.mode_info = mode_info;
            }
            Event::SessionUpdate(session_infos, resurrectable_sessions) => {
                let selected_session_name = self
                    .selected_session_idx
                    .and_then(|idx| self.sessions.get(idx))
                    .map(|entry| entry.name().to_owned());

                // Build unified session list
                let mut entries: Vec<SessionEntry> =
                    session_infos.into_iter().map(SessionEntry::Live).collect();

                if self.style.show_resurrectable {
                    for (name, duration) in resurrectable_sessions {
                        entries.push(SessionEntry::Resurrectable(name, duration));
                    }
                }

                let active_idx = entries.iter().position(|e| e.is_current()).unwrap_or(0);

                self.active_session_idx = active_idx;
                self.sessions = entries;
                if self.mode_info.mode == InputMode::Session {
                    self.selected_session_idx = selected_session_name
                        .and_then(|name| self.sessions.iter().position(|e| e.name() == name))
                        .or(Some(self.active_session_idx));
                }
                should_render = true;
            }
            Event::Mouse(me) => match me {
                Mouse::LeftClick(row, _col) => {
                    if let Some(entry) = self.get_session_at_row(row as usize) {
                        switch_session(Some(entry.name()));
                    }
                }
                Mouse::ScrollUp(_) => {
                    if self.active_session_idx > 0 {
                        let prev = self.active_session_idx - 1;
                        if let Some(entry) = self.sessions.get(prev) {
                            switch_session(Some(entry.name()));
                        }
                    }
                }
                Mouse::ScrollDown(_) => {
                    let next = self.active_session_idx + 1;
                    if next < self.sessions.len() {
                        if let Some(entry) = self.sessions.get(next) {
                            switch_session(Some(entry.name()));
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        should_render
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        match pipe_message.name.as_str() {
            "set_selectable" => {
                match pipe_message.payload.as_deref() {
                    Some("true") => {
                        self.is_selectable = true;
                        set_selectable(true);
                    }
                    Some("false") => {
                        self.is_selectable = false;
                        set_selectable(false);
                    }
                    _ => {}
                }
                false
            }
            "toggle_selectable" => {
                self.is_selectable = !self.is_selectable;
                set_selectable(self.is_selectable);
                false
            }
            "select_previous_session" | "zellij_vertical_sessions_select_previous" => {
                self.select_previous_session();
                true
            }
            "select_next_session" | "zellij_vertical_sessions_select_next" => {
                self.select_next_session();
                true
            }
            "confirm_session_selection" | "zellij_vertical_sessions_confirm_selection" => {
                self.confirm_session_selection();
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.last_rows = rows;

        if !self.permissions_granted || self.sessions.is_empty() {
            return;
        }

        self.render_vertical(rows, cols);
    }
}

impl State {
    fn select_previous_session(&mut self) {
        if self.mode_info.mode != InputMode::Session || self.sessions.is_empty() {
            return;
        }
        let current = self
            .selected_session_idx
            .unwrap_or(self.active_session_idx)
            .min(self.sessions.len() - 1);
        self.selected_session_idx = Some(current.saturating_sub(1));
    }

    fn select_next_session(&mut self) {
        if self.mode_info.mode != InputMode::Session || self.sessions.is_empty() {
            return;
        }
        let current = self
            .selected_session_idx
            .unwrap_or(self.active_session_idx)
            .min(self.sessions.len() - 1);
        self.selected_session_idx = Some((current + 1).min(self.sessions.len() - 1));
    }

    fn confirm_session_selection(&mut self) {
        if self.mode_info.mode != InputMode::Session {
            return;
        }

        let selected_name = self
            .selected_session_idx
            .and_then(|idx| self.sessions.get(idx))
            .map(|entry| entry.name().to_owned());
        self.selected_session_idx = None;
        switch_to_input_mode(&InputMode::Normal);

        if let Some(selected_name) = selected_name
            && self
                .sessions
                .get(self.active_session_idx)
                .is_none_or(|active| active.name() != selected_name)
        {
            switch_session(Some(&selected_name));
        }
    }

    fn expand_overflow_format(&self, format: &str, count: usize) -> String {
        format.replace("{count}", &count.to_string())
    }

    /// Expand a tmux-style format string with session info, returning styled text
    fn expand_format(&self, format: &str, entry: &SessionEntry, index: usize) -> StyledText {
        let tokens = parse_tmux_format(format);
        let mut result = StyledText::new();
        let mut current_style = InlineStyle::default();

        // Build indicators
        let mut indicators = String::new();
        match entry {
            SessionEntry::Live(info) => {
                if info.is_current_session {
                    indicators.push_str(&self.style.indicator_active);
                }
                if info.connected_clients > 1 {
                    let clients_str = self
                        .style
                        .indicator_clients
                        .replace("{n}", &info.connected_clients.to_string());
                    indicators.push_str(&clients_str);
                }
            }
            SessionEntry::Resurrectable(_, _) => {}
        }

        // Session details
        let (tab_count, client_count) = match entry {
            SessionEntry::Live(info) => (info.tabs.len(), info.connected_clients),
            SessionEntry::Resurrectable(_, _) => (0, 0),
        };

        for token in tokens {
            match token {
                FormatToken::Style(style) => {
                    current_style = style;
                }
                FormatToken::Variable { name, width } => {
                    let value = match name.as_str() {
                        "index" | "i" => index.to_string(),
                        "name" | "n" => sanitize_terminal_text(entry.name()),
                        "tabs" | "tab_count" => tab_count.to_string(),
                        "clients" | "client_count" | "c" => client_count.to_string(),
                        "indicators" => indicators.clone(),
                        "active" => {
                            if entry.is_current() {
                                self.style.indicator_active.clone()
                            } else {
                                String::new()
                            }
                        }
                        "type" => {
                            if entry.is_resurrectable() {
                                "dead".to_string()
                            } else {
                                "live".to_string()
                            }
                        }
                        _ => format!("{{{}}}", name),
                    };

                    let text = if let Some(w) = width {
                        truncate_string(&value, w)
                    } else {
                        truncate_string(&value, self.style.max_name_length)
                    };

                    result.push(text, current_style.clone());
                }
                FormatToken::Literal(text) => {
                    result.push(text, current_style.clone());
                }
            }
        }

        result
    }

    fn build_line(&self, content: &StyledText, cols: usize, is_selected: bool) -> String {
        let border = parse_styled_string(&self.style.border);
        let border_width = border.display_width();
        let effective_cols = cols.saturating_sub(border_width);
        let content = content.truncate(effective_cols);
        let content_width = content.display_width();
        let padding_needed = effective_cols.saturating_sub(content_width);

        let mut line = String::new();

        let has_fill = is_selected && content.segments.iter().any(|s| s.style.fill);

        if has_fill {
            line.push_str("\x1b[7m");
            for segment in &content.segments {
                let mut swapped_style = segment.style.clone();
                std::mem::swap(&mut swapped_style.fg, &mut swapped_style.bg);
                swapped_style.fill = false;
                if swapped_style.has_any_style() {
                    line.push_str("\x1b[0m\x1b[7m");
                    line.push_str(&swapped_style.to_ansi());
                }
                line.push_str(&segment.text);
            }
            if padding_needed > 0 {
                line.push_str(&" ".repeat(padding_needed));
            }
            line.push_str("\x1b[0m");
        } else {
            line.push_str(&content.to_ansi());
            if padding_needed > 0 {
                line.push_str(&" ".repeat(padding_needed));
            }
        }

        if border_width > 0 {
            line.push_str(&border.to_ansi());
        }

        line
    }

    fn build_empty_line(&self, cols: usize) -> String {
        let border = parse_styled_string(&self.style.border);
        let border_width = border.display_width();
        if border_width == 0 {
            return " ".repeat(cols);
        }
        let effective_cols = cols.saturating_sub(border_width);
        let mut line = " ".repeat(effective_cols);
        line.push_str(&border.to_ansi());
        line
    }

    fn render_vertical(&mut self, rows: usize, cols: usize) {
        let top_padding = self.style.padding_top.min(rows);
        let available_rows = rows.saturating_sub(top_padding);

        let session_count = self.sessions.len();
        let focused_index = self.selected_session_idx.unwrap_or(self.active_session_idx);

        let (start_index, end_index, sessions_above, sessions_below) =
            calculate_visible_range(session_count, available_rows, focused_index);

        let mut lines: Vec<String> = Vec::with_capacity(rows);

        // Top padding
        for _ in 0..top_padding {
            lines.push(self.build_empty_line(cols));
        }

        // Overflow above indicator
        if sessions_above > 0 {
            let indicator_text =
                self.expand_overflow_format(&self.style.overflow_above, sessions_above);
            let styled = parse_styled_string(&indicator_text);
            lines.push(self.build_line(&styled, cols, false));
        }

        // Render visible sessions
        for i in start_index..end_index {
            if let Some(entry) = self.sessions.get(i).cloned() {
                let is_current = entry.is_current();
                let is_selected = self.selected_session_idx == Some(i);
                let format = if is_selected {
                    &self.style.format_selected
                } else if is_current {
                    &self.style.format_active
                } else if entry.is_resurrectable() {
                    &self.style.format_resurrectable
                } else {
                    &self.style.format
                };

                let styled = self.expand_format(format, &entry, i + self.style.start_index);
                lines.push(self.build_line(&styled, cols, is_current || is_selected));
            }
        }

        // Overflow below indicator
        if sessions_below > 0 {
            let indicator_text =
                self.expand_overflow_format(&self.style.overflow_below, sessions_below);
            let styled = parse_styled_string(&indicator_text);
            lines.push(self.build_line(&styled, cols, false));
        }

        // Fill remaining rows
        while lines.len() < rows {
            lines.push(self.build_empty_line(cols));
        }

        // Print all lines
        for (i, line) in lines.iter().enumerate() {
            if i < lines.len() - 1 {
                println!("{}\x1b[m", line);
            } else {
                print!("{}\x1b[m", line);
            }
        }
    }

    fn get_session_at_row(&self, row: usize) -> Option<&SessionEntry> {
        if self.sessions.is_empty() {
            return None;
        }

        let session_count = self.sessions.len();
        let focused_index = self.selected_session_idx.unwrap_or(self.active_session_idx);
        let top_padding = self.style.padding_top.min(self.last_rows);
        if row < top_padding {
            return None;
        }
        let available_rows = self.last_rows.saturating_sub(top_padding);
        let (start_index, end_index, sessions_above, _) =
            calculate_visible_range(session_count, available_rows, focused_index);

        let row = row - top_padding;
        let content_start_row = if sessions_above > 0 { 1 } else { 0 };

        if sessions_above > 0 && row == 0 {
            // Clicked on overflow above indicator - go to previous session
            let target = start_index.saturating_sub(1);
            return self.sessions.get(target);
        }

        let row_in_content = row.saturating_sub(content_start_row);
        let clicked_index = start_index + row_in_content;

        if clicked_index < end_index && clicked_index < session_count {
            return self.sessions.get(clicked_index);
        }

        None
    }
}

fn calculate_visible_range(
    total: usize,
    available_rows: usize,
    active_index: usize,
) -> (usize, usize, usize, usize) {
    if total == 0 {
        return (0, 0, 0, 0);
    }
    if total <= available_rows {
        return (0, total, 0, 0);
    }

    let max_visible = available_rows.saturating_sub(2);
    if max_visible == 0 {
        return (0, 0, total, 0);
    }

    let mut start_index = active_index;
    let mut end_index = active_index + 1;
    let mut room_left = max_visible.saturating_sub(1);
    let mut alternate = false;

    while room_left > 0 {
        if !alternate && start_index > 0 {
            start_index -= 1;
            room_left -= 1;
        } else if alternate && end_index < total {
            end_index += 1;
            room_left -= 1;
        } else if start_index > 0 {
            start_index -= 1;
            room_left -= 1;
        } else if end_index < total {
            end_index += 1;
            room_left -= 1;
        } else {
            break;
        }
        alternate = !alternate;
    }

    (
        start_index,
        end_index,
        start_index,
        total.saturating_sub(end_index),
    )
}

fn truncate_string(s: &str, max_width: usize) -> String {
    if s.width() <= max_width {
        return s.to_string();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let mut truncated = String::new();
    let mut width = 0;
    for ch in s.chars() {
        let ch_width = ch.to_string().width();
        if width + ch_width + 3 > max_width {
            truncated.push_str("...");
            break;
        }
        truncated.push(ch);
        width += ch_width;
    }
    truncated
}
