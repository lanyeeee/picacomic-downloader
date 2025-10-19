<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import AppContent from './AppContent.vue'
import { GlobalThemeOverrides } from 'naive-ui'

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#DB547CFF',
    primaryColorHover: '#E87D9AFF',
    primaryColorPressed: '#B53C64FF',
    primaryColorSuppl: '#E87D9AFF',
  },
  Tabs: {
    tabTextColorActiveSegment: '#DB547CFF',
    tabTextColorHoverSegment: '#E87D9AFF',
    tabColorSegment: '#FFDAE9FF',
    colorSegment: '#FFFFFFFF',
  },
}
</script>

<template>
  <n-config-provider :theme-overrides="themeOverrides">
    <n-modal-provider>
      <n-notification-provider placement="bottom-right" :max="3">
        <n-message-provider>
          <app-content />
        </n-message-provider>
      </n-notification-provider>
    </n-modal-provider>
  </n-config-provider>
</template>
