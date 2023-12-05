use scraper::selector::CssLocalName;
use scraper::{CaseSensitivity, Element, Html, Selector};

#[derive(Clone, Debug)]
pub struct Example {
    pub data: String,
    pub part2_data: Option<String>,
    pub part1_answer: String,
    pub part2_answer: Option<String>,
}

impl Example {
    pub fn parse_example(html: String) -> Option<Self> {
        let document = Html::parse_document(&html);
        let article_selector = Selector::parse("article.day-desc *").ok()?;
        let articles = document.select(&article_selector);

        let mut found_for_example = false;
        let mut found_example = false;
        let mut part2 = false;
        let mut example: Option<String> = None;
        let mut part2_example: Option<String> = None;
        let mut answer: Option<String> = None;
        let mut part2_answer: Option<String> = None;

        for element in articles {
            match element.value().name() {
                "p" => {
                    let inner = element.inner_html().to_lowercase();
                    if inner.contains("for example") || (inner.contains("example") && !inner.contains("above") && !inner.contains("this") && !inner.contains("again")) {
                        found_for_example = true
                    }
                }
                "pre" => {
                    if !found_example
                        && found_for_example
                        && element.children().collect::<Vec<_>>().len() == 1
                    {
                        if let Some(child) = element.first_element_child() {
                            if child.value().name() == "code" {
                                if part2 {
                                    part2_example = Some(child.inner_html());
                                } else {
                                    example = Some(child.inner_html());
                                }
                                found_example = true;
                            }
                        }
                    }
                }
                "code" => {
                    if element.children().collect::<Vec<_>>().len() == 1 {
                        if let Some(child) = element.first_element_child() {
                            if child.value().name() == "em" {
                                if part2 {
                                    part2_answer = Some(child.inner_html())
                                } else {
                                    answer = Some(child.inner_html())
                                }
                            }
                        }
                    }
                }
                "h2" => {
                    if element.has_id(
                        &CssLocalName::from("part2"),
                        CaseSensitivity::AsciiCaseInsensitive,
                    ) {
                        part2 = true;
                        found_example = false;
                    }
                }
                _ => {}
            }
        }

        match (example, answer) {
            (Some(example), Some(answer)) => Some(Example {
                data: example,
                part2_data: part2_example,
                part1_answer: answer,
                part2_answer,
            }),
            _ => None,
        }
    }
}
