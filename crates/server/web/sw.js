"use strict";
// Service worker mínimo: guarda la interfaz para abrirla al instante y sin conexión.
// Nunca toca la API ni el WebSocket: los datos siempre vienen del servidor.
const CACHE = "tm-shell-v2";
const SHELL = ["/", "/i18n.js", "/app.js", "/canvas.js", "/style.css", "/manifest.webmanifest", "/icon-192.png"];

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting()));
});
self.addEventListener("activate", (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});
self.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (e.request.method !== "GET" || url.origin !== location.origin || !SHELL.includes(url.pathname)) return;
  // primero la red (así se actualiza con cada versión del servidor); sin red, lo guardado
  e.respondWith(fetch(e.request).then((r) => {
    const copy = r.clone();
    caches.open(CACHE).then((c) => c.put(e.request, copy));
    return r;
  }).catch(() => caches.match(e.request)));
});
