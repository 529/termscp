//! ## AuthActivity
//!
//! `auth_activity` is the module which implements the authentication activity

/**
 * MIT License
 *
 * termscp - Copyright (c) 2021 Christian Visintin
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
// Locals
use super::{AuthActivity, Context, FileTransferProtocol, InputMask};
use crate::filetransfer::params::ProtocolParams;
use crate::filetransfer::FileTransferParams;
use crate::ui::components::bookmark_list::{BookmarkList, BookmarkListPropsBuilder};
use crate::utils::ui::draw_area_in;
// Ext
use tui_realm_stdlib::{
    Input, InputPropsBuilder, Label, LabelPropsBuilder, List, ListPropsBuilder, Paragraph,
    ParagraphPropsBuilder, Radio, RadioPropsBuilder, Span, SpanPropsBuilder, Textarea,
    TextareaPropsBuilder,
};
use tuirealm::tui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{BorderType, Borders, Clear},
};
use tuirealm::{
    props::{Alignment, InputType, PropsBuilder, TableBuilder, TextSpan},
    Msg, Payload, Value,
};

impl AuthActivity {
    /// ### init
    ///
    /// Initialize view, mounting all startup components inside the view
    pub(super) fn init(&mut self) {
        let key_color = self.theme().misc_keys;
        let addr_color = self.theme().auth_address;
        let protocol_color = self.theme().auth_protocol;
        let port_color = self.theme().auth_port;
        let username_color = self.theme().auth_username;
        let password_color = self.theme().auth_password;
        let bookmarks_color = self.theme().auth_bookmarks;
        let recents_color = self.theme().auth_recents;
        // Headers
        self.view.mount(
            super::COMPONENT_TEXT_H1,
            Box::new(Label::new(
                LabelPropsBuilder::default()
                    .bold()
                    .italic()
                    .with_text(String::from("$ termscp"))
                    .build(),
            )),
        );
        self.view.mount(
            super::COMPONENT_TEXT_H2,
            Box::new(Label::new(
                LabelPropsBuilder::default()
                    .bold()
                    .italic()
                    .with_text(format!("$ version {}", env!("CARGO_PKG_VERSION")))
                    .build(),
            )),
        );
        // Footer
        self.view.mount(
            super::COMPONENT_TEXT_FOOTER,
            Box::new(Span::new(
                SpanPropsBuilder::default()
                    .with_spans(vec![
                        TextSpan::new("Press ").bold(),
                        TextSpan::new("<CTRL+H>").bold().fg(key_color),
                        TextSpan::new(" to show keybindings; ").bold(),
                        TextSpan::new("<CTRL+C>").bold().fg(key_color),
                        TextSpan::new(" to enter setup").bold(),
                    ])
                    .build(),
            )),
        );
        // Get default protocol
        let default_protocol: FileTransferProtocol = self.context().config().get_default_protocol();
        // Protocol
        self.view.mount(
            super::COMPONENT_RADIO_PROTOCOL,
            Box::new(Radio::new(
                RadioPropsBuilder::default()
                    .with_color(protocol_color)
                    .with_inverted_color(Color::Black)
                    .with_borders(Borders::ALL, BorderType::Rounded, protocol_color)
                    .with_title("Protocol", Alignment::Left)
                    .with_options(&["SFTP", "SCP", "FTP", "FTPS", "AWS S3"])
                    .with_value(Self::protocol_enum_to_opt(default_protocol))
                    .rewind(true)
                    .build(),
            )),
        );
        // Address
        self.view.mount(
            super::COMPONENT_INPUT_ADDR,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(addr_color)
                    .with_borders(Borders::ALL, BorderType::Rounded, addr_color)
                    .with_label("Remote host", Alignment::Left)
                    .build(),
            )),
        );
        // Port
        self.view.mount(
            super::COMPONENT_INPUT_PORT,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(port_color)
                    .with_borders(Borders::ALL, BorderType::Rounded, port_color)
                    .with_label("Port number", Alignment::Left)
                    .with_input(InputType::Number)
                    .with_input_len(5)
                    .with_value(Self::get_default_port_for_protocol(default_protocol).to_string())
                    .build(),
            )),
        );
        // Username
        self.view.mount(
            super::COMPONENT_INPUT_USERNAME,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(username_color)
                    .with_borders(Borders::ALL, BorderType::Rounded, username_color)
                    .with_label("Username", Alignment::Left)
                    .build(),
            )),
        );
        // Password
        self.view.mount(
            super::COMPONENT_INPUT_PASSWORD,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(password_color)
                    .with_borders(Borders::ALL, BorderType::Rounded, password_color)
                    .with_label("Password", Alignment::Left)
                    .with_input(InputType::Password)
                    .build(),
            )),
        );
        // Bucket
        self.view.mount(
            super::COMPONENT_INPUT_S3_BUCKET,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(addr_color)
                    .with_borders(Borders::ALL, BorderType::Rounded, addr_color)
                    .with_label("Bucket name", Alignment::Left)
                    .build(),
            )),
        );
        // Region
        self.view.mount(
            super::COMPONENT_INPUT_S3_REGION,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(port_color)
                    .with_borders(Borders::ALL, BorderType::Rounded, port_color)
                    .with_label("Region", Alignment::Left)
                    .build(),
            )),
        );
        // Profile
        self.view.mount(
            super::COMPONENT_INPUT_S3_PROFILE,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(username_color)
                    .with_borders(Borders::ALL, BorderType::Rounded, username_color)
                    .with_label("Profile", Alignment::Left)
                    .build(),
            )),
        );
        // Version notice
        if let Some(version) = self
            .context()
            .store()
            .get_string(super::STORE_KEY_LATEST_VERSION)
        {
            let version: String = version.to_string();
            self.view.mount(
                super::COMPONENT_TEXT_NEW_VERSION,
                Box::new(Span::new(
                    SpanPropsBuilder::default()
                        .with_foreground(Color::Yellow)
                        .with_spans(vec![
                            TextSpan::from("termscp "),
                            TextSpan::new(version.as_str()).underlined().bold(),
                            TextSpan::from(" is NOW available! Install update and view release notes with <CTRL+R>"),
                        ])
                        .build(),
                )),
            );
        }
        // Bookmarks
        self.view.mount(
            super::COMPONENT_BOOKMARKS_LIST,
            Box::new(BookmarkList::new(
                BookmarkListPropsBuilder::default()
                    .with_background(bookmarks_color)
                    .with_foreground(Color::Black)
                    .with_borders(Borders::ALL, BorderType::Plain, bookmarks_color)
                    .with_title("Bookmarks", Alignment::Left)
                    .build(),
            )),
        );
        // Recents
        self.view.mount(
            super::COMPONENT_RECENTS_LIST,
            Box::new(BookmarkList::new(
                BookmarkListPropsBuilder::default()
                    .with_background(recents_color)
                    .with_foreground(Color::Black)
                    .with_borders(Borders::ALL, BorderType::Plain, recents_color)
                    .with_title("Recent connections", Alignment::Left)
                    .build(),
            )),
        );
        // Load bookmarks
        let _ = self.view_bookmarks();
        let _ = self.view_recent_connections();
        // Active protocol
        self.view.active(super::COMPONENT_RADIO_PROTOCOL);
    }

    /// ### view
    ///
    /// Display view on canvas
    pub(super) fn view(&mut self) {
        let mut ctx: Context = self.context.take().unwrap();
        let _ = ctx.terminal().draw(|f| {
            // Check window size
            let height: u16 = f.size().height;
            self.check_minimum_window_size(height);
            // Prepare chunks
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(21), // Auth Form
                        Constraint::Min(3),     // Bookmarks
                    ]
                    .as_ref(),
                )
                .split(f.size());
            // Create explorer chunks
            let auth_chunks = Layout::default()
                .constraints(
                    [
                        Constraint::Length(1),                      // h1
                        Constraint::Length(1),                      // h2
                        Constraint::Length(1),                      // Version
                        Constraint::Length(3),                      // protocol
                        Constraint::Length(self.input_mask_size()), // Input mask
                        Constraint::Length(3),                      // footer
                    ]
                    .as_ref(),
                )
                .direction(Direction::Vertical)
                .split(chunks[0]);
            // Input mask chunks
            let input_mask = match self.input_mask() {
                InputMask::AwsS3 => Layout::default()
                    .constraints(
                        [
                            Constraint::Length(3), // bucket
                            Constraint::Length(3), // region
                            Constraint::Length(3), // profile
                        ]
                        .as_ref(),
                    )
                    .direction(Direction::Vertical)
                    .split(auth_chunks[4]),
                InputMask::Generic => Layout::default()
                    .constraints(
                        [
                            Constraint::Length(3), // host
                            Constraint::Length(3), // port
                            Constraint::Length(3), // username
                            Constraint::Length(3), // password
                        ]
                        .as_ref(),
                    )
                    .direction(Direction::Vertical)
                    .split(auth_chunks[4]),
            };
            // Create bookmark chunks
            let bookmark_chunks = Layout::default()
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .direction(Direction::Horizontal)
                .split(chunks[1]);
            // Render
            // Auth chunks
            self.view
                .render(super::COMPONENT_TEXT_H1, f, auth_chunks[0]);
            self.view
                .render(super::COMPONENT_TEXT_H2, f, auth_chunks[1]);
            self.view
                .render(super::COMPONENT_TEXT_NEW_VERSION, f, auth_chunks[2]);
            self.view
                .render(super::COMPONENT_RADIO_PROTOCOL, f, auth_chunks[3]);
            // Render input mask
            match self.input_mask() {
                InputMask::AwsS3 => {
                    self.view
                        .render(super::COMPONENT_INPUT_S3_BUCKET, f, input_mask[0]);
                    self.view
                        .render(super::COMPONENT_INPUT_S3_REGION, f, input_mask[1]);
                    self.view
                        .render(super::COMPONENT_INPUT_S3_PROFILE, f, input_mask[2]);
                }
                InputMask::Generic => {
                    self.view
                        .render(super::COMPONENT_INPUT_ADDR, f, input_mask[0]);
                    self.view
                        .render(super::COMPONENT_INPUT_PORT, f, input_mask[1]);
                    self.view
                        .render(super::COMPONENT_INPUT_USERNAME, f, input_mask[2]);
                    self.view
                        .render(super::COMPONENT_INPUT_PASSWORD, f, input_mask[3]);
                }
            }
            self.view
                .render(super::COMPONENT_TEXT_FOOTER, f, auth_chunks[5]);
            // Bookmark chunks
            self.view
                .render(super::COMPONENT_BOOKMARKS_LIST, f, bookmark_chunks[0]);
            self.view
                .render(super::COMPONENT_RECENTS_LIST, f, bookmark_chunks[1]);
            // Popups
            if let Some(props) = self.view.get_props(super::COMPONENT_TEXT_ERROR) {
                if props.visible {
                    let popup = draw_area_in(f.size(), 50, 10);
                    f.render_widget(Clear, popup);
                    // make popup
                    self.view.render(super::COMPONENT_TEXT_ERROR, f, popup);
                }
            }
            if let Some(props) = self.view.get_props(super::COMPONENT_TEXT_INFO) {
                if props.visible {
                    let popup = draw_area_in(f.size(), 50, 10);
                    f.render_widget(Clear, popup);
                    // make popup
                    self.view.render(super::COMPONENT_TEXT_INFO, f, popup);
                }
            }
            if let Some(props) = self.view.get_props(super::COMPONENT_TEXT_WAIT) {
                if props.visible {
                    let popup = draw_area_in(f.size(), 50, 10);
                    f.render_widget(Clear, popup);
                    // make popup
                    self.view.render(super::COMPONENT_TEXT_WAIT, f, popup);
                }
            }
            if let Some(props) = self.view.get_props(super::COMPONENT_TEXT_SIZE_ERR) {
                if props.visible {
                    let popup = draw_area_in(f.size(), 80, 20);
                    f.render_widget(Clear, popup);
                    // make popup
                    self.view.render(super::COMPONENT_TEXT_SIZE_ERR, f, popup);
                }
            }
            if let Some(props) = self.view.get_props(super::COMPONENT_RADIO_QUIT) {
                if props.visible {
                    // make popup
                    let popup = draw_area_in(f.size(), 30, 10);
                    f.render_widget(Clear, popup);
                    self.view.render(super::COMPONENT_RADIO_QUIT, f, popup);
                }
            }
            if let Some(props) = self
                .view
                .get_props(super::COMPONENT_RADIO_BOOKMARK_DEL_BOOKMARK)
            {
                if props.visible {
                    // make popup
                    let popup = draw_area_in(f.size(), 30, 10);
                    f.render_widget(Clear, popup);
                    self.view
                        .render(super::COMPONENT_RADIO_BOOKMARK_DEL_BOOKMARK, f, popup);
                }
            }
            if let Some(props) = self
                .view
                .get_props(super::COMPONENT_RADIO_BOOKMARK_DEL_RECENT)
            {
                if props.visible {
                    // make popup
                    let popup = draw_area_in(f.size(), 30, 10);
                    f.render_widget(Clear, popup);
                    self.view
                        .render(super::COMPONENT_RADIO_BOOKMARK_DEL_RECENT, f, popup);
                }
            }
            if let Some(props) = self.view.get_props(super::COMPONENT_TEXT_NEW_VERSION_NOTES) {
                if props.visible {
                    // make popup
                    let popup = draw_area_in(f.size(), 90, 85);
                    f.render_widget(Clear, popup);
                    let popup_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Percentage(90), // Notes
                                Constraint::Length(3),      // Install radio
                            ]
                            .as_ref(),
                        )
                        .split(popup);
                    self.view
                        .render(super::COMPONENT_TEXT_NEW_VERSION_NOTES, f, popup_chunks[0]);
                    self.view
                        .render(super::COMPONENT_RADIO_INSTALL_UPDATE, f, popup_chunks[1]);
                }
            }
            if let Some(props) = self.view.get_props(super::COMPONENT_TEXT_HELP) {
                if props.visible {
                    // make popup
                    let popup = draw_area_in(f.size(), 50, 70);
                    f.render_widget(Clear, popup);
                    self.view.render(super::COMPONENT_TEXT_HELP, f, popup);
                }
            }
            if let Some(props) = self
                .view
                .get_props(super::COMPONENT_RADIO_BOOKMARK_SAVE_PWD)
            {
                if props.visible {
                    // make popup
                    let popup = draw_area_in(f.size(), 20, 20);
                    f.render_widget(Clear, popup);
                    let popup_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Length(3), // Input form
                                Constraint::Length(2), // Yes/No
                            ]
                            .as_ref(),
                        )
                        .split(popup);
                    self.view
                        .render(super::COMPONENT_INPUT_BOOKMARK_NAME, f, popup_chunks[0]);
                    self.view
                        .render(super::COMPONENT_RADIO_BOOKMARK_SAVE_PWD, f, popup_chunks[1]);
                }
            }
        });
        self.context = Some(ctx);
    }

    // -- partials

    /// ### view_bookmarks
    ///
    /// Make text span from bookmarks
    pub(super) fn view_bookmarks(&mut self) -> Option<(String, Msg)> {
        let bookmarks: Vec<String> = self
            .bookmarks_list
            .iter()
            .map(|x| {
                Self::fmt_bookmark(
                    x,
                    self.bookmarks_client
                        .as_ref()
                        .unwrap()
                        .get_bookmark(x)
                        .unwrap(),
                )
            })
            .collect();
        match self.view.get_props(super::COMPONENT_BOOKMARKS_LIST) {
            None => None,
            Some(props) => {
                let msg = self.view.update(
                    super::COMPONENT_BOOKMARKS_LIST,
                    BookmarkListPropsBuilder::from(props)
                        .with_bookmarks(bookmarks)
                        .build(),
                );
                msg
            }
        }
    }

    /// ### view_recent_connections
    ///
    /// View recent connections
    pub(super) fn view_recent_connections(&mut self) -> Option<(String, Msg)> {
        let bookmarks: Vec<String> = self
            .recents_list
            .iter()
            .map(|x| {
                Self::fmt_recent(
                    self.bookmarks_client
                        .as_ref()
                        .unwrap()
                        .get_recent(x)
                        .unwrap(),
                )
            })
            .collect();
        match self.view.get_props(super::COMPONENT_RECENTS_LIST) {
            None => None,
            Some(props) => {
                let msg = self.view.update(
                    super::COMPONENT_RECENTS_LIST,
                    BookmarkListPropsBuilder::from(props)
                        .with_bookmarks(bookmarks)
                        .build(),
                );
                msg
            }
        }
    }

    // -- mount

    /// ### mount_error
    ///
    /// Mount error box
    pub(super) fn mount_error<S: AsRef<str>>(&mut self, text: S) {
        let err_color = self.theme().misc_error_dialog;
        self.mount_text_dialog(super::COMPONENT_TEXT_ERROR, text.as_ref(), err_color);
    }

    /// ### umount_error
    ///
    /// Umount error message
    pub(super) fn umount_error(&mut self) {
        self.view.umount(super::COMPONENT_TEXT_ERROR);
    }

    /// ### mount_info
    ///
    /// Mount info box
    pub(super) fn mount_info<S: AsRef<str>>(&mut self, text: S) {
        let color = self.theme().misc_info_dialog;
        self.mount_text_dialog(super::COMPONENT_TEXT_INFO, text.as_ref(), color);
    }

    /// ### umount_info
    ///
    /// Umount info message
    pub(super) fn umount_info(&mut self) {
        self.view.umount(super::COMPONENT_TEXT_INFO);
    }

    /// ### mount_error
    ///
    /// Mount wait box
    pub(super) fn mount_wait(&mut self, text: &str) {
        let wait_color = self.theme().misc_info_dialog;
        self.mount_text_dialog(super::COMPONENT_TEXT_WAIT, text, wait_color);
    }

    /// ### umount_wait
    ///
    /// Umount wait message
    pub(super) fn umount_wait(&mut self) {
        self.view.umount(super::COMPONENT_TEXT_WAIT);
    }

    /// ### mount_size_err
    ///
    /// Mount size error
    pub(super) fn mount_size_err(&mut self) {
        // Mount
        let err_color = self.theme().misc_error_dialog;
        self.mount_text_dialog(
            super::COMPONENT_TEXT_SIZE_ERR,
            "termscp requires at least 24 lines of height to run",
            err_color,
        );
    }

    /// ### umount_size_err
    ///
    /// Umount error size error
    pub(super) fn umount_size_err(&mut self) {
        self.view.umount(super::COMPONENT_TEXT_SIZE_ERR);
    }

    /// ### mount_quit
    ///
    /// Mount quit popup
    pub(super) fn mount_quit(&mut self) {
        // Protocol
        let quit_color = self.theme().misc_quit_dialog;
        self.mount_radio_dialog(
            super::COMPONENT_RADIO_QUIT,
            "Quit termscp?",
            &["Yes", "No"],
            0,
            quit_color,
        );
    }

    /// ### umount_quit
    ///
    /// Umount quit popup
    pub(super) fn umount_quit(&mut self) {
        self.view.umount(super::COMPONENT_RADIO_QUIT);
    }

    /// ### mount_bookmark_del_dialog
    ///
    /// Mount bookmark delete dialog
    pub(super) fn mount_bookmark_del_dialog(&mut self) {
        let warn_color = self.theme().misc_warn_dialog;
        self.mount_radio_dialog(
            super::COMPONENT_RADIO_BOOKMARK_DEL_BOOKMARK,
            "Delete bookmark?",
            &["Yes", "No"],
            1,
            warn_color,
        );
    }

    /// ### umount_bookmark_del_dialog
    ///
    /// umount delete bookmark dialog
    pub(super) fn umount_bookmark_del_dialog(&mut self) {
        self.view
            .umount(super::COMPONENT_RADIO_BOOKMARK_DEL_BOOKMARK);
    }

    /// ### mount_bookmark_del_dialog
    ///
    /// Mount recent delete dialog
    pub(super) fn mount_recent_del_dialog(&mut self) {
        let warn_color = self.theme().misc_warn_dialog;
        self.mount_radio_dialog(
            super::COMPONENT_RADIO_BOOKMARK_DEL_RECENT,
            "Delete bookmark?",
            &["Yes", "No"],
            1,
            warn_color,
        );
    }

    /// ### umount_recent_del_dialog
    ///
    /// umount delete recent dialog
    pub(super) fn umount_recent_del_dialog(&mut self) {
        self.view.umount(super::COMPONENT_RADIO_BOOKMARK_DEL_RECENT);
    }

    /// ### mount_bookmark_save_dialog
    ///
    /// Mount bookmark save dialog
    pub(super) fn mount_bookmark_save_dialog(&mut self) {
        let save_color = self.theme().misc_save_dialog;
        let warn_color = self.theme().misc_warn_dialog;
        self.view.mount(
            super::COMPONENT_INPUT_BOOKMARK_NAME,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_foreground(save_color)
                    .with_label("Save bookmark as…", Alignment::Center)
                    .with_borders(
                        Borders::TOP | Borders::RIGHT | Borders::LEFT,
                        BorderType::Rounded,
                        Color::Reset,
                    )
                    .build(),
            )),
        );
        self.view.mount(
            super::COMPONENT_RADIO_BOOKMARK_SAVE_PWD,
            Box::new(Radio::new(
                RadioPropsBuilder::default()
                    .with_color(warn_color)
                    .with_borders(
                        Borders::BOTTOM | Borders::RIGHT | Borders::LEFT,
                        BorderType::Rounded,
                        Color::Reset,
                    )
                    .with_title("Save password?", Alignment::Center)
                    .with_options(&[String::from("Yes"), String::from("No")])
                    .rewind(true)
                    .build(),
            )),
        );
        // Give focus to input bookmark name
        self.view.active(super::COMPONENT_INPUT_BOOKMARK_NAME);
    }

    /// ### umount_bookmark_save_dialog
    ///
    /// Umount bookmark save dialog
    pub(super) fn umount_bookmark_save_dialog(&mut self) {
        self.view.umount(super::COMPONENT_RADIO_BOOKMARK_SAVE_PWD);
        self.view.umount(super::COMPONENT_INPUT_BOOKMARK_NAME);
    }

    /// ### mount_help
    ///
    /// Mount help
    pub(super) fn mount_help(&mut self) {
        let key_color = self.theme().misc_keys;
        self.view.mount(
            super::COMPONENT_TEXT_HELP,
            Box::new(List::new(
                ListPropsBuilder::default()
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::White)
                    .with_highlighted_str(Some("?"))
                    .with_max_scroll_step(8)
                    .scrollable(true)
                    .bold()
                    .with_title("Help", Alignment::Center)
                    .with_rows(
                        TableBuilder::default()
                            .add_col(TextSpan::new("<ESC>").bold().fg(key_color))
                            .add_col(TextSpan::from("           Quit termscp"))
                            .add_row()
                            .add_col(TextSpan::new("<TAB>").bold().fg(key_color))
                            .add_col(TextSpan::from("           Switch from form and bookmarks"))
                            .add_row()
                            .add_col(TextSpan::new("<RIGHT/LEFT>").bold().fg(key_color))
                            .add_col(TextSpan::from("    Switch bookmark tab"))
                            .add_row()
                            .add_col(TextSpan::new("<UP/DOWN>").bold().fg(key_color))
                            .add_col(TextSpan::from("       Move up/down in current tab"))
                            .add_row()
                            .add_col(TextSpan::new("<ENTER>").bold().fg(key_color))
                            .add_col(TextSpan::from("         Connect/Load bookmark"))
                            .add_row()
                            .add_col(TextSpan::new("<DEL|E>").bold().fg(key_color))
                            .add_col(TextSpan::from("         Delete selected bookmark"))
                            .add_row()
                            .add_col(TextSpan::new("<CTRL+C>").bold().fg(key_color))
                            .add_col(TextSpan::from("        Enter setup"))
                            .add_row()
                            .add_col(TextSpan::new("<CTRL+S>").bold().fg(key_color))
                            .add_col(TextSpan::from("        Save bookmark"))
                            .build(),
                    )
                    .build(),
            )),
        );
        // Active help
        self.view.active(super::COMPONENT_TEXT_HELP);
    }

    /// ### umount_help
    ///
    /// Umount help
    pub(super) fn umount_help(&mut self) {
        self.view.umount(super::COMPONENT_TEXT_HELP);
    }

    /// ### mount_release_notes
    ///
    /// mount release notes text area
    pub(super) fn mount_release_notes(&mut self) {
        if let Some(ctx) = self.context.as_ref() {
            if let Some(release_notes) = ctx.store().get_string(super::STORE_KEY_RELEASE_NOTES) {
                // make spans
                let spans: Vec<TextSpan> = release_notes.lines().map(TextSpan::from).collect();
                let info_color = self.theme().misc_info_dialog;
                self.view.mount(
                    super::COMPONENT_TEXT_NEW_VERSION_NOTES,
                    Box::new(Textarea::new(
                        TextareaPropsBuilder::default()
                            .with_borders(Borders::ALL, BorderType::Rounded, info_color)
                            .with_title("Release notes", Alignment::Center)
                            .with_texts(spans)
                            .build(),
                    )),
                );
                // Mount install popup
                self.mount_radio_dialog(
                    super::COMPONENT_RADIO_INSTALL_UPDATE,
                    "Install new version?",
                    &["Yes", "No"],
                    0,
                    info_color,
                );
            }
        }
    }

    /// ### umount_release_notes
    ///
    /// Umount release notes text area
    pub(super) fn umount_release_notes(&mut self) {
        self.view.umount(super::COMPONENT_TEXT_NEW_VERSION_NOTES);
        self.view.umount(super::COMPONENT_RADIO_INSTALL_UPDATE);
    }

    /// ### get_protocol
    ///
    /// Get protocol from view
    pub(super) fn get_protocol(&self) -> FileTransferProtocol {
        self.get_input_protocol()
    }

    /// ### get_generic_params
    ///
    /// Collect input values from view
    pub(super) fn get_generic_params_input(&self) -> (String, u16, String, String) {
        let addr: String = self.get_input_addr();
        let port: u16 = self.get_input_port();
        let username: String = self.get_input_username();
        let password: String = self.get_input_password();
        (addr, port, username, password)
    }

    /// ### get_s3_params_input
    ///
    /// Collect s3 input values from view
    pub(super) fn get_s3_params_input(&self) -> (String, String, Option<String>) {
        let bucket: String = self.get_input_s3_bucket();
        let region: String = self.get_input_s3_region();
        let profile: Option<String> = self.get_input_s3_profile();
        (bucket, region, profile)
    }

    pub(super) fn get_input_addr(&self) -> String {
        match self.view.get_state(super::COMPONENT_INPUT_ADDR) {
            Some(Payload::One(Value::Str(x))) => x,
            _ => String::new(),
        }
    }

    pub(super) fn get_input_port(&self) -> u16 {
        match self.view.get_state(super::COMPONENT_INPUT_PORT) {
            Some(Payload::One(Value::Usize(x))) => match x > 65535 {
                true => 0,
                false => x as u16,
            },
            _ => 0,
        }
    }

    pub(super) fn get_input_protocol(&self) -> FileTransferProtocol {
        match self.view.get_state(super::COMPONENT_RADIO_PROTOCOL) {
            Some(Payload::One(Value::Usize(x))) => Self::protocol_opt_to_enum(x),
            _ => FileTransferProtocol::Sftp,
        }
    }

    pub(super) fn get_input_username(&self) -> String {
        match self.view.get_state(super::COMPONENT_INPUT_USERNAME) {
            Some(Payload::One(Value::Str(x))) => x,
            _ => String::new(),
        }
    }

    pub(super) fn get_input_password(&self) -> String {
        match self.view.get_state(super::COMPONENT_INPUT_PASSWORD) {
            Some(Payload::One(Value::Str(x))) => x,
            _ => String::new(),
        }
    }

    pub(super) fn get_input_s3_bucket(&self) -> String {
        match self.view.get_state(super::COMPONENT_INPUT_S3_BUCKET) {
            Some(Payload::One(Value::Str(x))) => x,
            _ => String::new(),
        }
    }

    pub(super) fn get_input_s3_region(&self) -> String {
        match self.view.get_state(super::COMPONENT_INPUT_S3_REGION) {
            Some(Payload::One(Value::Str(x))) => x,
            _ => String::new(),
        }
    }

    pub(super) fn get_input_s3_profile(&self) -> Option<String> {
        match self.view.get_state(super::COMPONENT_INPUT_S3_PROFILE) {
            Some(Payload::One(Value::Str(x))) => match x.is_empty() {
                true => None,
                false => Some(x),
            },
            _ => None,
        }
    }

    /// ### input_mask_size
    ///
    /// Returns the input mask size based on current input mask
    pub(super) fn input_mask_size(&self) -> u16 {
        match self.input_mask() {
            InputMask::AwsS3 => 9,
            InputMask::Generic => 12,
        }
    }

    /// ### fmt_bookmark
    ///
    /// Format bookmark to display on ui
    fn fmt_bookmark(name: &str, b: FileTransferParams) -> String {
        let addr: String = Self::fmt_recent(b);
        format!("{} ({})", name, addr)
    }

    /// ### fmt_recent
    ///
    /// Format recent connection to display on ui
    fn fmt_recent(b: FileTransferParams) -> String {
        let protocol: String = b.protocol.to_string().to_lowercase();
        match b.params {
            ProtocolParams::AwsS3(s3) => {
                let profile: String = match s3.profile {
                    Some(p) => format!("[{}]", p),
                    None => String::default(),
                };
                format!(
                    "{}://{} ({}) {}",
                    protocol, s3.bucket_name, s3.region, profile
                )
            }
            ProtocolParams::Generic(params) => {
                let username: String = match params.username {
                    None => String::default(),
                    Some(u) => format!("{}@", u),
                };
                format!(
                    "{}://{}{}:{}",
                    protocol, username, params.address, params.port
                )
            }
        }
    }

    // -- mount helpers

    fn mount_text_dialog(&mut self, id: &str, text: &str, color: Color) {
        // Mount
        self.view.mount(
            id,
            Box::new(Paragraph::new(
                ParagraphPropsBuilder::default()
                    .with_borders(Borders::ALL, BorderType::Thick, color)
                    .with_foreground(color)
                    .bold()
                    .with_text_alignment(Alignment::Center)
                    .with_texts(vec![TextSpan::from(text)])
                    .build(),
            )),
        );
        // Give focus to error
        self.view.active(id);
    }

    fn mount_radio_dialog(
        &mut self,
        id: &str,
        text: &str,
        opts: &[&str],
        default: usize,
        color: Color,
    ) {
        self.view.mount(
            id,
            Box::new(Radio::new(
                RadioPropsBuilder::default()
                    .with_color(color)
                    .with_inverted_color(Color::Black)
                    .with_borders(Borders::ALL, BorderType::Rounded, color)
                    .with_title(text, Alignment::Center)
                    .with_options(opts)
                    .with_value(default)
                    .rewind(true)
                    .build(),
            )),
        );
        // Active
        self.view.active(id);
    }
}
