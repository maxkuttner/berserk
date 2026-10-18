
use std::error::Error;

use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use ratatui_image::{
    picker::Picker,
    protocol::StatefulProtocol,
    Resize, StatefulImage,
};
use url::Url;

enum View {
    ChapterList,
    Reader,
}

struct App {
    // Navigation
    view: View,
    chapters: Vec<Url>,
    chapter_list_state: ListState,


    // Reader
    pages: Vec<Url>,
    current_page: usize,
    image: Option<StatefulProtocol>,
    picker: Picker,
}

impl App {
    async fn new(chapters: Vec<Url>) -> Result<Self, Box<dyn Error>> {
        // Detect graphics capabilities before entering TUI mode
        let picker = Picker::from_query_stdio()?;

        let mut chapter_list_state = ListState::default();

        if !chapters.is_empty() {
            chapter_list_state.select(Some(0));
        }

        Ok(Self {
            view: View::ChapterList,
            chapters,
            chapter_list_state,
            pages: Vec::new(),
            current_page: 0,
            image: None,
            picker,
        })
    }

    fn select_next_chapter(&mut self) {
        if self.chapters.is_empty() {
            return;
        }

        let current = self.chapter_list_state.selected().unwrap_or(0);
        let next = (current + 1) % self.chapters.len();

        self.chapter_list_state.select(Some(next));
    }

    fn select_previous_chapter(&mut self) {
        if self.chapters.is_empty() {
            return;
        }

        let current = self.chapter_list_state.selected().unwrap_or(0);

        let previous = if current == 0 {
            self.chapters.len() - 1
        } else {
            current - 1
        };

        self.chapter_list_state.select(Some(previous));
    }
    async fn open_selected_chapter(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(index) = self.chapter_list_state.selected() else {
            return Ok(());
        };

        let chapter_url = self.chapters[index].clone();

        let chapter = crate::berserk::fetch_chapter(
            chapter_url.as_str()
        ).await?;

        if chapter.pages.is_empty() {
            return Ok(());
        }

        self.pages = chapter.pages;
        self.current_page = 0;
        self.image = None;

        self.load_page().await?;

        self.view = View::Reader;

        Ok(())
    }

    async fn load_page(&mut self) -> Result<(), Box<dyn Error>> {
        let url = &self.pages[self.current_page];

        // CDN rejects hotlinked images (403) without a Referer from the site
        let bytes = reqwest::Client::new()
            .get(url.as_str())
            .header(reqwest::header::REFERER, "https://readberserk.com/")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let decoded = image::load_from_memory(&bytes)?;

        // Resized to the render area at draw time
        self.image = Some(self.picker.new_resize_protocol(decoded));

        Ok(())
    }

    async fn next_page(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page + 1 < self.pages.len() {
            self.current_page += 1;
            self.load_page().await?;
        }
        Ok(())
    }

    async fn previous_page(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.load_page().await?;
        }
        Ok(())
    }
}

fn render_chapter_list(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
) {
    let items: Vec<ListItem> = app
        .chapters
        .iter()
        .map(|url| {
            let name = url
                .path_segments()
                .and_then(|segments| segments.filter(|s| !s.is_empty()).last())
                .unwrap_or("Unknown chapter");

            ListItem::new(name.replace('-', " "))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Select Chapter")
                .borders(Borders::ALL)
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(
        list,
        area,
        &mut app.chapter_list_state,
    );
}

pub async fn run(chapters: Vec<Url>) -> Result<(), Box<dyn Error>> {
    let mut app = App::new(chapters).await?;

    let mut terminal = ratatui::init();

    let result = async {
        loop {
            terminal.draw(|frame| {
                match app.view {
                    View::ChapterList => {
                        let area = frame.area();
                        render_chapter_list(frame, area, &mut app);
                    }

                    View::Reader => {
                        let areas = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(1),
                                Constraint::Length(1),
                            ])
                            .split(frame.area());

                        if let Some(img) = &mut app.image {
                            frame.render_stateful_widget(
                                StatefulImage::default()
                                    .resize(Resize::Fit(None)),
                                areas[0],
                                img,
                            );
                        }

                        let status = format!(
                            "Page {}/{} | ←/→ Navigate | Esc Chapters | q Quit",
                            app.current_page + 1,
                            app.pages.len()
                        );

                        frame.render_widget(
                            Paragraph::new(status),
                            areas[1],
                        );
                    }
                }
            })?;

            if let Event::Key(key) = event::read()? {
                match app.view {
                    View::ChapterList => {
                        match key.code {
                            KeyCode::Char('q') => break,

                            KeyCode::Down | KeyCode::Char('j') => {
                                app.select_next_chapter();
                            }

                            KeyCode::Up | KeyCode::Char('k') => {
                                app.select_previous_chapter();
                            }

                            KeyCode::Enter => {
                                app.open_selected_chapter().await?;
                            }

                            _ => {}
                        }
                    }

                    View::Reader => {
                        match key.code {
                            KeyCode::Char('q') => break,

                            KeyCode::Esc => {
                                app.view = View::ChapterList;
                            }

                            KeyCode::Right | KeyCode::Char('l') => {
                                app.next_page().await?;
                            }

                            KeyCode::Left | KeyCode::Char('h') => {
                                app.previous_page().await?;
                            }

                            _ => {}
                        }
                    }
                }
            }
        }

        Ok::<(), Box<dyn Error>>(())
    }
        .await;

    ratatui::restore();
    result
}
