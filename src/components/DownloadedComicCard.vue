<script setup lang="ts">
import { Comic } from '../bindings.ts'
import { CurrentTabName } from '../types.ts'

const props = defineProps<{
  comic: Comic
}>()

const pickedComic = defineModel<Comic | undefined>('pickedComic', { required: true })
const currentTabName = defineModel<CurrentTabName>('currentTabName', { required: true })

function pickComic() {
  pickedComic.value = props.comic
  currentTabName.value = 'chapter'
}
</script>

<template>
  <n-card content-style="padding: 0.25rem;" hoverable>
    <div class="flex">
      <img
        class="w-24 aspect-[3/4] object-contain mr-4 cursor-pointer transform transition-transform duration-200 hover:scale-106"
        :src="`${comic.thumb.fileServer}/static/${comic.thumb.path}`"
        alt=""
        referrerpolicy="no-referrer"
        @click="pickComic" />
      <div class="flex flex-col w-full justify-between">
        <div class="flex flex-col">
          <span
            class="font-bold text-xl line-clamp-2 cursor-pointer transition-colors duration-200 hover:text-blue-5"
            @click="pickComic">
            {{ comic.title }}
          </span>
          <span class="text-red">作者：{{ comic.author }}</span>
          <span class="text-gray" v-html="`分类：${comic.categories}`"></span>
        </div>
      </div>
    </div>
  </n-card>
</template>
