<template>
  <el-dialog
    v-model="dialogVisible"
    :title="fileName || '文件预览'"
    :width="dialogWidth"
    destroy-on-close
    @close="handleClose"
  >
    <div class="preview-container" v-loading="loading" element-loading-text="加载中...">
      <!-- Images -->
      <div v-if="fileCategory === 'image'" class="preview-image">
        <img :src="mediaUrl" :alt="fileName" @load="loading = false" @error="handleLoadError" />
      </div>

      <!-- PDF -->
      <div v-else-if="fileCategory === 'pdf'" class="preview-pdf">
        <VuePdfEmbed :src="pdfSrc" @loaded="loading = false" @error="handleLoadError" />
      </div>

      <!-- Word -->
      <div v-else-if="fileCategory === 'word'" class="preview-office">
        <VueOfficeDocx
          :src="officeSrc"
          @rendered="loading = false"
          @error="handleOfficeError"
        />
      </div>

      <!-- Excel -->
      <div v-else-if="fileCategory === 'excel'" class="preview-office">
        <VueOfficeExcel
          :src="officeSrc"
          @rendered="loading = false"
          @error="handleOfficeError"
        />
      </div>

      <!-- PowerPoint -->
      <div v-else-if="fileCategory === 'ppt'" class="preview-office">
        <VueOfficePptx
          :src="officeSrc"
          @rendered="loading = false"
          @error="handleOfficeError"
        />
      </div>

      <!-- Video -->
      <div v-else-if="fileCategory === 'video'" class="preview-video">
        <video :src="mediaUrl" controls @loadeddata="loading = false" @error="handleLoadError">
          您的浏览器不支持视频播放
        </video>
      </div>

      <!-- Audio -->
      <div v-else-if="fileCategory === 'audio'" class="preview-audio">
        <audio :src="mediaUrl" controls @canplay="loading = false" @error="handleLoadError">
          您的浏览器不支持音频播放
        </audio>
      </div>

      <!-- Text/Code -->
      <div v-else-if="fileCategory === 'text'" class="preview-text">
        <pre v-if="textContent">{{ textContent }}</pre>
      </div>

      <!-- Unsupported -->
      <div v-else class="preview-unsupported">
        <el-empty description="不支持预览此文件类型">
          <el-button type="primary" @click="downloadFile">下载文件</el-button>
        </el-empty>
      </div>
    </div>
  </el-dialog>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import axios from 'axios'
import VuePdfEmbed from 'vue-pdf-embed'
import VueOfficeDocx from '@vue-office/docx'
import '@vue-office/docx/lib/index.css'
import VueOfficeExcel from '@vue-office/excel'
import '@vue-office/excel/lib/index.css'
import VueOfficePptx from '@vue-office/pptx'
import { getPreviewUrl } from '@/api/file'

const props = defineProps({
  visible: {
    type: Boolean,
    default: false
  },
  fileId: {
    type: [String, Number],
    default: null
  },
  fileName: {
    type: String,
    default: ''
  },
  fileType: {
    type: String,
    default: null
  }
})

const emit = defineEmits(['update:visible'])

const loading = ref(false)
const textContent = ref('')
const officeSrc = ref(null)
const pdfSrc = ref(null)
/** 图片/视频/音频直接用 URL，由后端 serve 路由或 S3 预签名提供 */
const mediaUrl = ref('')

// --- Dialog v-model ---
const dialogVisible = computed({
  get: () => props.visible,
  set: (val) => emit('update:visible', val)
})

// --- File extension helpers ---
const fileExtension = computed(() => {
  if (!props.fileName) return ''
  const parts = props.fileName.split('.')
  return parts.length > 1 ? parts.pop().toLowerCase() : ''
})

const IMAGE_EXTS = ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'bmp', 'avif', 'ico']
const VIDEO_EXTS = ['mp4', 'webm', 'ogg', 'mov', 'avi']
const AUDIO_EXTS = ['mp3', 'wav', 'ogg', 'aac', 'flac', 'm4a']
const TEXT_EXTS = [
  'txt', 'log', 'md', 'csv', 'json', 'xml', 'yml', 'yaml', 'toml', 'ini',
  'html', 'css', 'js', 'ts', 'vue', 'rs', 'py', 'java', 'go', 'c', 'cpp',
  'sh', 'bat', 'sql'
]

const fileCategory = computed(() => {
  const ext = fileExtension.value
  if (!ext) return 'unknown'
  if (IMAGE_EXTS.includes(ext)) return 'image'
  if (ext === 'pdf') return 'pdf'
  if (ext === 'docx') return 'word'
  if (ext === 'xlsx' || ext === 'xls') return 'excel'
  if (ext === 'pptx' || ext === 'ppt') return 'ppt'
  if (VIDEO_EXTS.includes(ext)) return 'video'
  if (AUDIO_EXTS.includes(ext)) return 'audio'
  if (TEXT_EXTS.includes(ext)) return 'text'
  return 'unknown'
})

const dialogWidth = computed(() => {
  switch (fileCategory.value) {
    case 'image': return '70vw'
    case 'pdf': return '80vw'
    case 'video': return '70vw'
    case 'audio': return '500px'
    case 'text': return '70vw'
    case 'excel': return '90vw'
    case 'word': return '80vw'
    case 'ppt': return '90vw'
    default: return '500px'
  }
})

// --- Auth helper ---
function getAuthHeaders() {
  const token = localStorage.getItem('token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

// --- Load content when dialog opens ---
watch(
  () => props.visible,
  async (val) => {
    if (!val || !props.fileId) return

    loading.value = true
    textContent.value = ''
    officeSrc.value = null
    pdfSrc.value = null
    mediaUrl.value = ''

    const category = fileCategory.value

    try {
      // 获取预览地址
      const previewRes = await getPreviewUrl(String(props.fileId))
      const url = previewRes.data.url

      if (category === 'image' || category === 'video' || category === 'audio') {
        // 直接用 URL，浏览器自带缓存（本地永久 / S3 预签名内有效）
        mediaUrl.value = url
        loading.value = false
      } else if (category === 'pdf') {
        pdfSrc.value = url
        loading.value = false
      } else if (category === 'word' || category === 'excel' || category === 'ppt') {
        // @vue-office 需要 ArrayBuffer，走带 auth 的 axios 请求
        const resp = await axios.get(url, {
          responseType: 'arraybuffer',
          headers: getAuthHeaders()
        })
        officeSrc.value = resp.data
        loading.value = false
      } else if (category === 'text') {
        const resp = await axios.get(url, {
          responseType: 'text',
          headers: getAuthHeaders()
        })
        textContent.value = typeof resp.data === 'string' ? resp.data : JSON.stringify(resp.data, null, 2)
        loading.value = false
      } else {
        loading.value = false
      }
    } catch (err) {
      console.error('文件加载失败:', err)
      loading.value = false
    }
  }
)

// --- Error handlers ---
function handleLoadError() {
  loading.value = false
  console.error('资源加载失败')
}

function handleOfficeError(err) {
  loading.value = false
  console.error('Office 文件渲染失败:', err)
}

// --- Cleanup ---
function handleClose() {
  if (mediaUrl.value) {
    mediaUrl.value = ''
  }
  textContent.value = ''
  officeSrc.value = null
  pdfSrc.value = null
  loading.value = false
}

// --- Download fallback ---
function downloadFile() {
  if (!props.fileId) return
  const a = document.createElement('a')
  a.href = `/api/file/${props.fileId}/download`
  a.download = props.fileName || ''
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
}
</script>

<style scoped>
.preview-container {
  min-height: 200px;
  max-height: 80vh;
  overflow: auto;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-image img {
  max-width: 100%;
  max-height: 80vh;
  object-fit: contain;
}

.preview-pdf {
  width: 100%;
  max-height: 80vh;
  overflow: auto;
}

.preview-office {
  width: 100%;
  max-height: 80vh;
  overflow: auto;
}

.preview-video video {
  max-width: 100%;
  max-height: 75vh;
}

.preview-audio {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
}

.preview-audio audio {
  width: 80%;
}

.preview-text {
  width: 100%;
  max-height: 75vh;
  overflow: auto;
}

.preview-text pre {
  margin: 0;
  padding: 16px;
  font-family: 'Cascadia Code', 'Fira Code', 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
  background: #f5f7fa;
  border-radius: 4px;
}

.preview-unsupported {
  padding: 40px 0;
}
</style>
