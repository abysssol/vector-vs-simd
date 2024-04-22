use std::{env, fs, iter};

use serde::{Deserialize, Serialize};
use serde_json as json;

fn main() {
    let path = env::args().nth(1).expect("needs argument to path");
    let raw_json =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("error reading file {path}: {err}"));
    let mut article_json: Vec<Paragraph> =
        json::from_str(&raw_json).unwrap_or_else(|err| panic!("{err}"));
    for paragraph in &mut article_json {
        if let Some(metadata) = paragraph.metadata.as_mut() {
            let prefix = "ImageMetadata:";
            if metadata.image_ref.starts_with(prefix) {
                metadata.image_ref.replace_range(0..prefix.len(), "");
            }
        };

        for markup in &mut paragraph.markups {
            if let Some(href) = markup.href.as_mut() {
                let prefix = "https://web.archive.org/web/";
                if href.starts_with(prefix) {
                    href.replace_range(0..prefix.len(), "");
                    let i = href.find('/').unwrap();
                    href.replace_range(0..i + '/'.len_utf8(), "");
                }
            };
        }
    }

    let start_len = article_json
        .iter()
        .fold(0, |acc, paragraph| acc + paragraph.text.len());
    let html: String = article_json
        .iter()
        .scan(false, |is_list, paragraph| {
            let mut buf = String::with_capacity(paragraph.text.len());

            let html = match paragraph.html {
                HtmlTag::Paragraph => Html::Paragraph,
                HtmlTag::Header3 => Html::Header3,
                HtmlTag::Header4 => Html::Header4,
                HtmlTag::Code => Html::Code,
                HtmlTag::Preformatted => Html::Preformatted,
                HtmlTag::ListItem => Html::ListItem,
                HtmlTag::Quote => Html::Quote,
                HtmlTag::Link => unreachable!(),
                HtmlTag::Image => {
                    let image = &paragraph
                        .metadata
                        .as_ref()
                        .expect("an image ref should be provided if the html type is `img`")
                        .image_ref;
                    Html::Image(format!("https://miro.medium.com/v2/format:webp/{image}"))
                }
            };

            if paragraph.markups.is_empty() {
                buf.push_str(&html.tag(TagType::Open));
                buf.push_str(&paragraph.text);
                buf.push_str(&html.tag(TagType::Close));
            } else {
                let markups: Vec<_> = paragraph
                    .markups
                    .iter()
                    .map(|markup| {
                        (
                            markup,
                            match markup.html {
                                HtmlTag::Paragraph => Html::Paragraph,
                                HtmlTag::Header3 => Html::Header3,
                                HtmlTag::Header4 => Html::Header4,
                                HtmlTag::Code => Html::Code,
                                HtmlTag::Preformatted => Html::Preformatted,
                                HtmlTag::ListItem => Html::ListItem,
                                HtmlTag::Quote => Html::Quote,
                                HtmlTag::Link => Html::Link(
                                    markup
                                        .href
                                        .as_ref()
                                        .expect(
                                            "an href should be provided if the html type is `a`",
                                        )
                                        .clone(),
                                ),
                                HtmlTag::Image => unreachable!(),
                            },
                        )
                    })
                    .collect();

                let mut tags: Vec<(usize, &Html, TagType)> = markups
                    .iter()
                    .flat_map(|(markup, markup_html)| {
                        [
                            (markup.start, markup_html, TagType::Open),
                            (markup.end, markup_html, TagType::Close),
                        ]
                    })
                    .collect();
                tags.sort_unstable_by_key(|(key, _, _)| *key);

                buf.push_str(&paragraph.text);
                for &(i, markup_html, tag_type) in tags.iter().rev() {
                    let i = buf
                        .char_indices()
                        .map(|(i, _)| i)
                        .chain(iter::once(buf.len()))
                        .nth(i)
                        .unwrap();
                    buf.insert_str(i, &markup_html.tag(tag_type));
                }

                buf.insert_str(0, &html.tag(TagType::Open));
                buf.push_str(&html.tag(TagType::Close));
            }

            if let HtmlTag::ListItem = paragraph.html {
                if !*is_list {
                    *is_list = true;
                    buf.insert_str(0, "<ul>");
                }
            } else if *is_list {
                *is_list = false;
                buf.insert_str(0, "</ul>");
            }

            Some(buf)
        })
        .fold(String::with_capacity(start_len), |mut buf, html| {
            buf.push_str(&html);
            buf.push('\n');
            buf
        });

    println!(
        r#"<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="utf-8">
  <title>Vector vs SIMD Intructions</title>
  <style>
    body {{
      background-color: black;
      color: white;
      margin-left: 25%;
      margin-right: 25%;
    }}
    figure {{
      display: block;
      margin-left: 20%;
      margin-right: 20%;
      text-align: center;
    }}
    blockquote {{
      color: #FFF0D8;
      background-color: #101018;
      font-style: italic;
      border-left: .2em solid #606058;
      padding-left: 1em;
    }}
    blockquote:before {{
      content: '“';
    }}
    blockquote:after {{
      content: '”';
    }}
    a {{
      padding-left: 0.2em;
      padding-right: 0.2em;
    }}
    a:link {{
      color: #40D0FF;
    }}
    a:visited {{
      color: #A050E0;
    }}
    a:hover {{
      background-color: #202020;
      border-radius: 0.4em;
    }}
  </style>
</head>

<body>
{html}
</body>

</html>"#
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Paragraph {
    text: String,
    #[serde(rename = "type")]
    html: HtmlTag,
    markups: Vec<Markup>,

    layout: Option<Layout>,
    metadata: Option<Metadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Markup {
    start: usize,
    end: usize,
    #[serde(rename = "type")]
    html: HtmlTag,
    href: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum HtmlTag {
    #[serde(rename = "P")]
    Paragraph,
    #[serde(rename = "H3")]
    Header3,
    #[serde(rename = "H4")]
    Header4,
    #[serde(rename = "CODE")]
    Code,
    #[serde(rename = "PRE")]
    Preformatted,
    #[serde(rename = "ULI")]
    ListItem,
    #[serde(rename = "BQ")]
    Quote,
    #[serde(rename = "A")]
    Link,
    #[serde(rename = "IMG")]
    Image,
}

#[derive(Debug, Clone)]
enum Html {
    Paragraph,
    Header3,
    Header4,
    Code,
    Preformatted,
    ListItem,
    Quote,
    Link(String),
    Image(String),
}

#[derive(Debug, Clone, Copy)]
enum TagType {
    Open,
    Close,
}

impl Html {
    fn tag(&self, tag_type: TagType) -> String {
        let slash = match tag_type {
            TagType::Open => "",
            TagType::Close => "/",
        };
        match self {
            Self::Paragraph => format!("<{slash}p>"),
            Self::Header3 => format!("<{slash}h3>"),
            Self::Header4 => format!("<{slash}h4>"),
            Self::Code => format!("<{slash}code>"),
            Self::Preformatted => format!("<{slash}pre>"),
            Self::ListItem => format!("<{slash}li>"),
            Self::Quote => format!("<{slash}blockquote>"),
            Self::Link(href) => match tag_type {
                TagType::Open => format!("<a href=\"{href}\">"),
                TagType::Close => "</a>".to_owned(),
            },
            Self::Image(src) => match tag_type {
                TagType::Open => format!("<figure><img src=\"{src}\"><figcaption>"),
                TagType::Close => "</figcaption></figure>".to_owned(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Layout {
    #[serde(rename = "INSET_CENTER")]
    InsetCenter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Metadata {
    #[serde(rename = "__ref")]
    image_ref: String,
}
