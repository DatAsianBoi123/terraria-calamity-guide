use std::collections::HashMap;

use num_traits::{ToPrimitive, FromPrimitive};
use poise::{serenity_prelude::{Http, UserId, User, ButtonStyle, Color, CreateEmbed, Timestamp, CreateActionRow, CreateButton, CreateEmbedFooter}, ChoiceParameter};
use rand::Rng;
use sqlx::{PgPool, types::{BigDecimal, chrono::{NaiveDateTime, Utc}}};
use tracing::info;

use crate::loadout_data::{CalamityClass, Stage};

type RawIssue = (i32, BigDecimal, i16, i16, String, String, NaiveDateTime);

#[derive(Debug)]
pub struct NoIssueFound(pub i32);

#[non_exhaustive]
pub struct Issues {
    pub issues: HashMap<i32, Issue>,
}

impl Issues {
    pub async fn create(&mut self, author: &User, class: CalamityClass, stage: Stage, incorrect: String, correct: String, pool: &PgPool) -> &Issue {
        let mut id = rand::thread_rng().gen_range(0..i32::MAX);
        while self.issues.contains_key(&id) { id = rand::thread_rng().gen_range(0..i32::MAX); }

        let issue = Issue {
            id,
            author: author.clone(),
            class,
            stage,
            incorrect,
            correct,
            created_at: Utc::now().naive_utc(),
        };

        sqlx::query("INSERT INTO issues(id, author, class, stage, incorrect, correct, created_at) VALUES($1, $2, $3, $4, $5, $6, $7)")
            .bind(issue.id)
            .bind(BigDecimal::from_u64(issue.author.id.get()).expect("author is valid big decimal"))
            .bind(issue.class as i16)
            .bind(issue.stage as i16)
            .bind(&issue.incorrect)
            .bind(&issue.correct)
            .bind(issue.created_at)
            .execute(pool).await.expect("query is correct");

        info!("{} created an issue", issue.author.name);

        self.issues.insert(id, issue);
        self.issues.get(&id).expect("issue exists")
    }

    pub async fn resolve(&mut self, id: i32, pool: &PgPool) -> Result<Issue, NoIssueFound> {
        if !self.issues.contains_key(&id) { return Err(NoIssueFound(id)) }

        sqlx::query("DELETE FROM issues WHERE id = $1")
            .bind(id)
            .execute(pool).await.expect("query is valid");

        self.issues.remove(&id).ok_or(NoIssueFound(id))
    }

    pub async fn load(http: &Http, pool: &PgPool) -> Self {
        let mut issues = HashMap::new();

        let issue_array: Vec<RawIssue> = sqlx::query_as("SELECT * FROM issues")
            .fetch_all(pool).await.expect("query is correct");

        for raw_issue in issue_array {
            let author = raw_issue.1.to_u64().map(|id| UserId::new(id).to_user(http)).expect("id is u64").await.expect("author is user");
            let class = FromPrimitive::from_i16(raw_issue.2).expect("class number is valid class");
            let stage = FromPrimitive::from_i16(raw_issue.3).expect("stage number is valid stage");
            let issue = Issue {
                id: raw_issue.0,
                author,
                class,
                stage,
                incorrect: raw_issue.4,
                correct: raw_issue.5,
                created_at: Utc::now().naive_utc(),
            };
            issues.insert(raw_issue.0, issue);
        }

        Issues {
            issues,
        }
    }
}

pub struct Issue {
    pub id: i32,
    pub author: User,
    pub class: CalamityClass,
    pub stage: Stage,
    pub incorrect: String,
    pub correct: String,
    pub created_at: NaiveDateTime,
}

impl Issue {
    pub fn create_embed(&self) -> CreateEmbed {
        CreateEmbed::default()
            .title(format!("Issue {:x}", self.id))
            .field("Class", self.class.name(), true)
            .field("Stage", self.stage.name(), true)
            .field("** **", "** **", false)
            .field("Incorrect Phrase", &self.incorrect, true)
            .field("Correct Phrase", &self.correct, true)
            .color(Color::ORANGE)
            .footer(CreateEmbedFooter::new(format!("Created by {}", self.author.name)))
            .timestamp(Timestamp::from_unix_timestamp(self.created_at.and_utc().timestamp()).expect("timestamp is valid"))
    }

    pub fn create_components(&self) -> Vec<CreateActionRow> {
        vec![
            CreateActionRow::Buttons(vec![
                CreateButton::new(format!("r-{}", self.id)).style(ButtonStyle::Success).label("Resolve"),
            ]),
        ]
    }

    pub fn create_resolved_embed(&self) -> CreateEmbed {
        self.create_embed()
            .title(format!("Resolved {:x}", self.id))
            .color(Color::from_rgb(21, 209, 49))
    }
}

