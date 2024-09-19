<script setup lang="ts">
import {ComicInSearch, Pagination} from "../bindings.ts";

defineProps<{
  comicInSearchPagination: Pagination<ComicInSearch> | undefined;
  onClickItem: (comicId: string) => void;
}>();
</script>

<template>
  <div v-if="comicInSearchPagination!==undefined" class="flex flex-col gap-row-2 overflow-auto">
    <n-card class="cursor-pointer"
            content-style="padding: 0.25rem;"
            hoverable
            v-for="comicInSearch in comicInSearchPagination.docs"
            :key="comicInSearch._id"
            @click="onClickItem(comicInSearch._id)">
      <div class="flex">
        <img class="w-24 object-cover pr-4"
             :src="`${comicInSearch.thumb.fileServer}/static/${comicInSearch.thumb.path}`"
             alt=""
             referrerpolicy="no-referrer"/>
        <div class="flex flex-col h-full">
          <span class="font-bold text-xl">{{ comicInSearch.title }}</span>
          <span class="text-red">作者：{{ comicInSearch.author }}</span>
          <span class="text-gray" v-html="`分类：${comicInSearch.categories}`"></span>
        </div>
      </div>
    </n-card>
  </div>
</template>