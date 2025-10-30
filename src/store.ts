import { defineStore } from 'pinia'
import { CurrentTabName, ProgressData } from './types.ts'
import { Comic, Config, GetFavoriteResult, SearchResult, UserProfileDetailRespData, GetRankResult } from './bindings.ts'
import { ref } from 'vue'
import { ProgressesPaneTabName } from './panes/ProgressesPane/ProgressesPane.vue'

export const useStore = defineStore('store', () => {
  const config = ref<Config>()
  const userProfile = ref<UserProfileDetailRespData>()
  const pickedComic = ref<Comic>()
  const currentTabName = ref<CurrentTabName>('search')
  const progresses = ref<Map<string, ProgressData>>(new Map())
  const getFavoriteResult = ref<GetFavoriteResult>()
  const searchResult = ref<SearchResult>()
  const getRankResult = ref<GetRankResult>()
  const progressesPaneTabName = ref<ProgressesPaneTabName>('uncompleted')

  return {
    config,
    userProfile,
    pickedComic,
    currentTabName,
    progresses,
    getFavoriteResult,
    searchResult,
    getRankResult,
    progressesPaneTabName,
  }
})
