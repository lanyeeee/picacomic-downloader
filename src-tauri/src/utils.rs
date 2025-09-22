use std::sync::Arc;

use anyhow::Context;
use parking_lot::Mutex;
use tauri::AppHandle;
use tokio::task::JoinSet;

use crate::{extensions::AnyhowErrorToStringChain, pica_client::PicaClient, types::Comic};

pub fn filename_filter(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | '/' => ' ',
            ':' => '：',
            '*' => '⭐',
            '?' => '？',
            '"' => '\'',
            '<' => '《',
            '>' => '》',
            '|' => '丨',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub async fn get_comic(
    app: &AppHandle,
    pica_client: &PicaClient,
    comic_id: &str,
) -> anyhow::Result<Comic> {
    // 获取漫画详情和章节的第一页
    let (comic, first_page) = tokio::try_join!(
        pica_client.get_comic(comic_id),
        pica_client.get_chapter(comic_id, 1)
    )
    .context(format!("获取漫画`{comic_id}`的详情或章节的第1页失败"))?;
    // 准备根据章节的第一页获取所有章节
    // 先把第一页的章节放进去
    // TODO: 在join_set里返回chapter_page.docs，然后在.join_next()里处理，这样就不用锁了
    let chapters = Arc::new(Mutex::new(vec![]));
    chapters.lock().extend(first_page.docs);
    // 获取剩下的章节
    let total_pages = first_page.pages;
    let mut join_set = JoinSet::new();
    for page in 2..=total_pages {
        let pica_client = pica_client.clone();
        let chapters = chapters.clone();
        let comic_id = comic_id.to_string();
        // 创建获取章节的任务
        join_set.spawn(async move {
            let chapter_page = match pica_client.get_chapter(&comic_id, page).await {
                Ok(chapter_page) => chapter_page,
                Err(err) => {
                    let err_title = format!("获取漫画`{comic_id}`章节的第{page}页失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);
                    return;
                }
            };
            chapters.lock().extend(chapter_page.docs);
        });
    }
    // 等待所有章节获取完毕
    join_set.join_all().await;
    // 按章节顺序排序
    let chapters = {
        let mut chapters = chapters.lock();
        chapters.sort_by_key(|chapter| chapter.order);
        std::mem::take(&mut *chapters)
    };
    let comic = Comic::from(app, comic, chapters)?;

    Ok(comic)
}
