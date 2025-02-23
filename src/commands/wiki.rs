use std::time::Duration;

use poise::{command, serenity_prelude::{ActionRowComponent, ButtonKind, ButtonStyle, Color, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, EditInteractionResponse, Timestamp}, ChoiceParameter, CreateReply};
use reqwest::{header::{HeaderMap, HeaderValue}, redirect::Policy, Client, IntoUrl, Url};
use scraper::{ElementRef, Html, Node, Selector};

use crate::{Context, PoiseResult};

macro_rules! build_some {
    ($ident: ident; $pat: pat = $item: expr => $expr: expr) => {
        if let Some($pat) = $item {
            $ident = $expr;
        }
    };
}

pub struct WikiPage {
    pub wiki: WikiType,
    pub url: Url,
    pub item_name: String,
    pub description: String,
}

impl From<Html> for WikiPage {
    fn from(value: Html) -> Self {
        let title = value.select(&Selector::parse("title").expect("valid selector"))
            .next().expect("title exists")
            .inner_html();

        let wiki = WikiType::from_wiki_title(&title);
        let item_name = value.select(&Selector::parse("#firstHeading span").expect("valid selector"))
            .next().expect("first heading exists")
            .inner_html();
        let url = value.select(&Selector::parse("head link[rel=canonical]").expect("valid selector"))
            .next().expect("url exists")
            .value()
            .attr("href").expect("href exists")
            .parse().expect("href is a valid url");
        let description = value.select(&Selector::parse("head meta[name=description]").expect("valid selector"))
            .next().expect("description exists")
            .value()
            .attr("content").expect("content exists")
            .to_string();

        WikiPage {
            wiki,
            item_name,
            url,
            description,
        }
    }
}

#[derive(ChoiceParameter)]
pub enum WikiType {
    Vanilla,
    Calamity,
}

impl WikiType {
    pub fn url(&self) -> Url {
        match self {
            Self::Vanilla => Url::parse("https://terraria.wiki.gg").expect("valid url"),
            Self::Calamity => Url::parse("https://calamitymod.wiki.gg").expect("valid url"),
        }
    }

    pub fn wiki_name(&self) -> &str {
        match self {
            Self::Vanilla => "Terraria Wiki",
            Self::Calamity => "Calamity Wiki",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::Vanilla => Color::from_rgb(53, 232, 101),
            Self::Calamity => Color::from_rgb(222, 56, 27),
        }
    }

    pub fn from_wiki_title(title: &str) -> Self {
        if title.contains("Terraria") { Self::Vanilla }
        else { Self::Calamity }
    }
}

#[command(
    slash_command,
    description_localized("en-US", "Views a specific item"),
)]
pub async fn wiki(
    ctx: Context<'_>,
    #[description = "The wiki to use"]
    wiki: WikiType,
    #[description = "The thing to search for"]
    search: String,
) -> PoiseResult {
    ctx.defer().await?;
    let url = Url::parse_with_params(wiki.url().join("/wiki/Special:Search")?.as_str(), &[("search", &search)])?;

    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10.9; rv:50.0) Gecko/20100101 Firefox/50.0"));
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .redirect(Policy::none()).build()?;

    let request = client.get(url.clone()).send().await?;
    if !request.status().is_redirection() {
        return handle_search_results(ctx, wiki, search, &client, request.text().await.expect("text")).await;
    }

    let redirect_url = request.headers().get("location").expect("location in heading").to_str().expect("location only includes ASCII");

    ctx.send(handle_wiki_redirect(&client, redirect_url).await?).await?;
    Ok(())
}

async fn handle_wiki_redirect(client: &Client, url: impl IntoUrl) -> Result<CreateReply, crate::Error> {
    let request = client.get(url).send().await?;

    let WikiPage { wiki, url, item_name, description } = Html::parse_document(&request.text().await.expect("text")).into();

    let reply = CreateReply::default().embed(CreateEmbed::new()
        .author(CreateEmbedAuthor::new(wiki.wiki_name().to_string()))
        .url(url)
        .title(item_name)
        .description(description)
        .color(wiki.color())
        .timestamp(Timestamp::now()));
    Ok(reply)
}

async fn handle_search_results(ctx: Context<'_>, wiki: WikiType, search: String, client: &Client, document: String) -> PoiseResult {
    let reply = ctx.send(create_search_reply(&search, wiki, Html::parse_document(&document))).await?;
    let message = reply.message().await?;
    match message.await_component_interaction(&ctx.serenity_context().shard).timeout(Duration::from_secs(30)).await {
        Some(interaction) => {
            interaction.defer(ctx).await?;
            let create_reply = handle_wiki_redirect(client, &interaction.data.custom_id).await?;

            let mut edit_response = EditInteractionResponse::new()
                .embeds(create_reply.embeds)
                .components(create_reply.components.unwrap_or_default());
            build_some!(edit_response; content = create_reply.content => edit_response.content(content));
            interaction.edit_response(ctx, edit_response).await?;
        },
        None => {
            let edited = message.into_owned();
            let mut create_reply = CreateReply::default()
                .content(edited.content);
            let components = edited.components.into_iter().map(|row| {
                let mut buttons = Vec::new();
                for component in row.components {
                    if let ActionRowComponent::Button(button) = component {
                        let mut create_button = match button.data {
                            ButtonKind::Link { url } => CreateButton::new_link(url),
                            ButtonKind::NonLink { custom_id, style } => CreateButton::new(custom_id).style(style),
                            ButtonKind::Premium { sku_id } => CreateButton::new_premium(sku_id),
                        }.disabled(true);
                        build_some!(create_button; label = button.label => create_button.label(label));
                        build_some!(create_button; emoji = button.emoji => create_button.emoji(emoji));
                        buttons.push(create_button);
                    }
                }
                CreateActionRow::Buttons(buttons)
            }).collect();
            create_reply = create_reply.components(components);

            for embed in edited.embeds {
                let mut create_embed = CreateEmbed::new();
                build_some!(create_embed; title = embed.title => create_embed.title(title));
                build_some!(create_embed; description = embed.description => create_embed.description(description));
                build_some!(create_embed; url = embed.url => create_embed.url(url));
                build_some!(create_embed; timestamp = embed.timestamp => create_embed.timestamp(timestamp));
                build_some!(create_embed; color = embed.colour => create_embed.color(color));
                build_some!(create_embed; footer = embed.footer => {
                    let mut create_footer = CreateEmbedFooter::new(footer.text);
                    build_some!(create_footer; icon_url = footer.icon_url => create_footer.icon_url(icon_url));
                    create_embed.footer(create_footer)
                });
                build_some!(create_embed; image = embed.image => create_embed.image(image.url));
                build_some!(create_embed; thumbnail = embed.thumbnail => create_embed.thumbnail(thumbnail.url));
                build_some!(create_embed; author = embed.author => {
                    let mut create_author = CreateEmbedAuthor::new(author.name);
                    build_some!(create_author; url = author.url => create_author.url(url));
                    build_some!(create_author; icon_url = author.icon_url => create_author.icon_url(icon_url));
                    create_embed.author(create_author)
                });
                for field in embed.fields {
                    create_embed = create_embed.field(field.name, field.value, field.inline);
                }
                create_reply = create_reply.embed(create_embed);
            }

            reply.edit(ctx, create_reply).await?;
        },
    };
    Ok(())
}

fn create_search_reply(search: &str, wiki: WikiType, html: Html) -> CreateReply {
    let url = html.select(&Selector::parse("head link[rel=canonical]").expect("valid selector"))
        .next().expect("url exists")
        .value()
        .attr("href").expect("href exists");
    let search_results_selector = Selector::parse("div.searchresults ul.mw-search-results li").expect("valid selector");
    let search_results: Vec<(&str, Url, String)> = html.select(&search_results_selector).take(5)
            .map(|element| {
                let search_anchor = element.select(&Selector::parse("div.mw-search-result-heading a").expect("valid selector"))
                    .next().expect("search result header exists")
                    .value();
                let search_description = element.select(&Selector::parse("div.searchresult").expect("valid selector"))
                    .next().expect("search result exists");

                let title = search_anchor.attr("title").expect("title attr exists");
                let url = search_anchor.attr("href").expect("href attr exists");
                let url = wiki.url().join(url).expect("join url");
                let description = search_description.children()
                    .fold(String::new(), |mut prev, curr| {
                        let node = curr.value();

                        match node {
                            Node::Text(text) => prev += text,
                            Node::Element(_) => prev += &format!("**{}**", ElementRef::wrap(curr).expect("node is element").inner_html()),
                            _ => {},
                        };

                        prev
                    });

                (title, url, description)
            })
            .collect();

    let embed = CreateEmbed::new()
        .url(url)
        .title(format!("Search Results for '{search}'"))
        .fields(search_results.iter().map(|(title, _, description)| (*title, description, false)))
        .color(Color::BLUE)
        .timestamp(Timestamp::now());

    let buttons = search_results.into_iter()
        .map(|(title, url, _)| {
            CreateButton::new(url.to_string()).label(title).style(ButtonStyle::Secondary)
        })
        .collect();
    let button_row = CreateActionRow::Buttons(buttons);

    CreateReply::default()
        .embed(embed)
        .components(vec![button_row])
}

