use poise::{command, ChoiceParameter};
use reqwest::Url;
use scraper::{Html, Selector};

use crate::{Context, PoiseResult};

pub struct WikiPage {
    pub wiki: WikiType,
    pub item_name: String,
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

        WikiPage {
            wiki,
            item_name,
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

    pub fn from_wiki_title(title: &str) -> Self {
        if title.contains("Terraria") { Self::Vanilla }
        else { Self::Calamity }
    }
}

#[command(
    slash_command,
    description_localized("en-US", "Views a specific item"),
    rename = "viewitem",
)]
pub async fn view_item(
    ctx: Context<'_>,
    #[description = "The wiki to use"]
    wiki: WikiType,
    #[description = "The item to search for"]
    item: String,
) -> PoiseResult {
    ctx.defer().await?;
    let url = Url::parse_with_params(wiki.url().join("index.php")?.as_str(), &[("search", item)])?;
    let request = reqwest::get(url.clone()).await?;
    let new_url = request.url();
    // did not get redirected
    if &url == new_url {
        ctx.say("item was not found").await?;
        return Ok(());
    }
    let wiki_page = get_wiki_page(&request.text().await.expect("text"));
    ctx.say(format!("{} in wiki {}", wiki_page.item_name, wiki.name())).await?;
    Ok(())
}

fn get_wiki_page(html: &str) -> WikiPage {
    Html::parse_document(html).into()
}

