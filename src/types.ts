import { DownloadTaskEvent, ImageRespData } from './bindings.ts'

export type ComicInfo = {
  _id: string
  title: string
  author?: string
  categories: string[]
  thumb: ImageRespData
}

export type CurrentTabName = 'search' | 'favorite' | 'downloaded' | 'chapter'

export type ProgressData = DownloadTaskEvent & { percentage: number; indicator: string }
