// @ts-nocheck
// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

/** user-defined commands **/


export const commands = {
async greet(name: string) : Promise<string> {
    return await TAURI_INVOKE("greet", { name });
},
async getConfig() : Promise<Config> {
    return await TAURI_INVOKE("get_config");
},
async saveConfig(config: Config) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("save_config", { config }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async login(email: string, password: string) : Promise<Result<string, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("login", { email, password }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getUserProfile() : Promise<Result<UserProfileDetailRespData, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("get_user_profile") };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async searchComic(keyword: string, sort: Sort, page: number, categories: string[]) : Promise<Result<Pagination<ComicInSearchRespData>, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("search_comic", { keyword, sort, page, categories }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getComic(comicId: string) : Promise<Result<Comic, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("get_comic", { comicId }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getEpisodeImage(comicId: string, episodeOrder: number, page: number) : Promise<Result<Pagination<EpisodeImageRespData>, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("get_episode_image", { comicId, episodeOrder, page }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async downloadEpisodes(episodes: Episode[]) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("download_episodes", { episodes }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async downloadComic(comicId: string) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("download_comic", { comicId }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async showPathInFileManager(path: string) : Promise<Result<null, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("show_path_in_file_manager", { path }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async getFavouriteComics(sort: Sort, page: number) : Promise<Result<Pagination<ComicInFavoriteRespData>, CommandError>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("get_favourite_comics", { sort, page }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
}
}

/** user-defined events **/


export const events = __makeEvents__<{
downloadEpisodeEndEvent: DownloadEpisodeEndEvent,
downloadEpisodePendingEvent: DownloadEpisodePendingEvent,
downloadEpisodeStartEvent: DownloadEpisodeStartEvent,
downloadImageErrorEvent: DownloadImageErrorEvent,
downloadImageSuccessEvent: DownloadImageSuccessEvent,
downloadSpeedEvent: DownloadSpeedEvent,
updateOverallDownloadProgressEvent: UpdateOverallDownloadProgressEvent
}>({
downloadEpisodeEndEvent: "download-episode-end-event",
downloadEpisodePendingEvent: "download-episode-pending-event",
downloadEpisodeStartEvent: "download-episode-start-event",
downloadImageErrorEvent: "download-image-error-event",
downloadImageSuccessEvent: "download-image-success-event",
downloadSpeedEvent: "download-speed-event",
updateOverallDownloadProgressEvent: "update-overall-download-progress-event"
})

/** user-defined constants **/



/** user-defined types **/

export type Comic = { _id: string; title: string; author?: string; pagesCount: number; episodes: Episode[]; epsCount: number; finished: boolean; categories: string[]; thumb: Image; likesCount: number; _creator: Creator; description?: string; chineseTeam?: string; tags: string[]; updated_at: string; created_at: string; allowDownload: boolean; viewsCount: number; isLiked: boolean; commentsCount: number }
export type ComicInFavoriteRespData = { _id: string; title: string; author?: string; pagesCount: number; epsCount: number; finished: boolean; categories: string[]; thumb: ImageRespData; likesCount: number }
export type ComicInSearchRespData = { _id: string; author?: string; categories: string[]; chineseTeam?: string; created_at: string; description?: string; finished: boolean; likesCount: number; tags: string[]; thumb: ImageRespData; title: string; totalLikes: number | null; totalViews: number | null; updated_at: string }
export type CommandError = string
export type Config = { token: string; downloadDir: string; episodeDownloadInterval: number; downloadWithAuthor: boolean }
export type Creator = { _id: string; gender: string; name: string; title: string; verified: boolean | null; exp: number; level: number; characters: string[]; avatar?: Image; slogan?: string; role: string; character?: string }
export type DownloadEpisodeEndEvent = DownloadEpisodeEndEventPayload
export type DownloadEpisodeEndEventPayload = { epId: string; errMsg: string | null }
export type DownloadEpisodePendingEvent = DownloadEpisodePendingEventPayload
export type DownloadEpisodePendingEventPayload = { epId: string; title: string }
export type DownloadEpisodeStartEvent = DownloadEpisodeStartEventPayload
export type DownloadEpisodeStartEventPayload = { epId: string; title: string; total: number }
export type DownloadImageErrorEvent = DownloadImageErrorEventPayload
export type DownloadImageErrorEventPayload = { epId: string; url: string; errMsg: string }
export type DownloadImageSuccessEvent = DownloadImageSuccessEventPayload
export type DownloadImageSuccessEventPayload = { epId: string; url: string; downloadedCount: number }
export type DownloadSpeedEvent = DownloadSpeedEventPayload
export type DownloadSpeedEventPayload = { speed: string }
export type Episode = { epId: string; epTitle: string; comicId: string; comicTitle: string; author: string; isDownloaded: boolean; order: number }
export type EpisodeImageRespData = { _id: string; media: ImageRespData }
export type Image = { originalName: string; path: string; fileServer: string }
export type ImageRespData = { originalName: string; path: string; fileServer: string }
export type Pagination<T> = { total: number; limit: number; page: number; pages: number; docs: T[] }
export type Sort = "Default" | "TimeNewest" | "TimeOldest" | "LikeMost" | "ViewMost"
export type UpdateOverallDownloadProgressEvent = UpdateOverallDownloadProgressEventPayload
export type UpdateOverallDownloadProgressEventPayload = { downloadedImageCount: number; totalImageCount: number; percentage: number }
export type UserProfileDetailRespData = { _id: string; gender: string; name: string; title: string; verified: boolean; exp: number; level: number; characters: string[]; avatar?: ImageRespData; birthday: string; email: string; created_at: string; isPunched: boolean }

/** tauri-specta globals **/

import {
	invoke as TAURI_INVOKE,
	Channel as TAURI_CHANNEL,
} from "@tauri-apps/api/core";
import * as TAURI_API_EVENT from "@tauri-apps/api/event";
import { type WebviewWindow as __WebviewWindow__ } from "@tauri-apps/api/webviewWindow";

type __EventObj__<T> = {
	listen: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.listen<T>>;
	once: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.once<T>>;
	emit: null extends T
		? (payload?: T) => ReturnType<typeof TAURI_API_EVENT.emit>
		: (payload: T) => ReturnType<typeof TAURI_API_EVENT.emit>;
};

export type Result<T, E> =
	| { status: "ok"; data: T }
	| { status: "error"; error: E };

function __makeEvents__<T extends Record<string, any>>(
	mappings: Record<keyof T, string>,
) {
	return new Proxy(
		{} as unknown as {
			[K in keyof T]: __EventObj__<T[K]> & {
				(handle: __WebviewWindow__): __EventObj__<T[K]>;
			};
		},
		{
			get: (_, event) => {
				const name = mappings[event as keyof T];

				return new Proxy((() => {}) as any, {
					apply: (_, __, [window]: [__WebviewWindow__]) => ({
						listen: (arg: any) => window.listen(name, arg),
						once: (arg: any) => window.once(name, arg),
						emit: (arg: any) => window.emit(name, arg),
					}),
					get: (_, command: keyof __EventObj__<any>) => {
						switch (command) {
							case "listen":
								return (arg: any) => TAURI_API_EVENT.listen(name, arg);
							case "once":
								return (arg: any) => TAURI_API_EVENT.once(name, arg);
							case "emit":
								return (arg: any) => TAURI_API_EVENT.emit(name, arg);
						}
					},
				});
			},
		},
	);
}
