/// <reference types="astro/client" />

interface ImportMetaEnv {
  /** Version du Lanceur, lue dans src-tauri/tauri.conf.json au build. */
  readonly LANCEUR_VERSION: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
