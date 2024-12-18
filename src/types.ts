import {ImageRespData} from "./bindings.ts";

export type ComicInfo = {
    _id: string,
    title: string,
    author?: string,
    categories: string[],
    thumb: ImageRespData,
}