import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

// Version unique : champ "version" de src-tauri/tauri.conf.json (0.1.0 si le fichier n'existe pas encore).
let version = '0.1.0';
try {
  const conf = JSON.parse(
    fs.readFileSync(fileURLToPath(new URL('./src-tauri/tauri.conf.json', import.meta.url)), 'utf8'),
  );
  if (typeof conf.version === 'string') version = conf.version;
} catch {
  /* fichier absent ou illisible : version par défaut */
}

// Site statique : Tauri sert directement dist/ (aucun serveur).
export default defineConfig({
  output: 'static',
  server: { port: 4752 },
  vite: {
    // Tauri compile dans src-tauri/target : ne pas le surveiller (EBUSY).
    server: { watch: { ignored: ['**/src-tauri/**'] } },
    plugins: [tailwindcss()],
    define: { 'import.meta.env.LANCEUR_VERSION': JSON.stringify(version) },
  },
});
