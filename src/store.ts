import { defineStore } from 'pinia'
import { CurrentTabName } from './types.ts'
import { Comic, Config, UserProfileDetailRespData } from './bindings.ts'
import { ref } from 'vue'

export const useStore = defineStore('store', () => {
  const config = ref<Config>()
  const userProfile = ref<UserProfileDetailRespData>()
  const pickedComic = ref<Comic>()
  const currentTabName = ref<CurrentTabName>('search')

  return { config, userProfile, pickedComic, currentTabName }
})
