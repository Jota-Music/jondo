pub(super) fn spanish(key: &str) -> &str {
    match key {
        "Settings" => "Ajustes",
        "Developed by" => "Desarrollado por",
        "Account" => "Cuenta",
        "Language" => "Idioma",
        "Background memory" => "Memoria en segundo plano",
        "Rooms" => "Salas",
        "About" => "Acerca de",
        "Backups" => "Copias de seguridad",
        "Logs" => "Registros",
        "Account, interface and backups" => "Cuenta, interfaz y copias de seguridad",
        "Back to playlists" => "Volver a las playlists",
        "Check" => "Comprobar",
        "Choose" => "Elegir",
        "Choose the language of the interface." => "Elige el idioma de la interfaz.",
        "Export" => "Exportar",
        "Folder" => "Carpeta",
        "Join" => "Unirse",
        "Leave" => "Salir",
        "No saved rooms." => "No hay salas guardadas.",
        "Playlist not found" => "Playlist no encontrada",
        "Save the application log to a file." => {
            "Guarda el registro de la aplicación en un archivo."
        }
        "Search in channel" => "Buscar en el canal",
        "Search in playlist" => "Buscar en la playlist",
        "Show what you are listening to in your Discord profile." => {
            "Muestra lo que estás escuchando en tu perfil de Discord."
        }
        "This playlist is no longer in your library." => {
            "Esta playlist ya no está en tu biblioteca."
        }
        "No playlists yet. Create one below." => "Aún no hay playlists. Crea una abajo.",
        "empty" => "vacía",
        "Empty queue." => "Cola vacía.",
        "Pick a playlist and click a track to start." => {
            "Elige una lista y haz clic en una canción para empezar."
        }
        "{count} followers" => "{count} seguidores",
        "Album not found" => "Álbum no encontrado",
        "Artist not found" => "Artista no encontrado",
        "Channel not found" => "Canal no encontrado",
        "User not found" => "Usuario no encontrado",
        "{count} selected" => "{count} seleccionados",
        "Connected as" => "Conectado como",
        "Disconnect" => "Desconectar",
        "Connect Spotify" => "Conectar Spotify",
        "Spotify is unavailable" => "Spotify no está disponible",
        "Connecting to Spotify…" => "Conectando con Spotify…",
        "Try again" => "Reintentar",
        "YouTube — needed for age-restricted videos and your own playlists" => {
            "YouTube — necesario para vídeos con restricción de edad y tus propias playlists"
        }
        "Signed in" => "Sesión iniciada",
        "Sign in" => "Iniciar sesión",
        "Sign out" => "Cerrar sesión",
        "Export cookies" => "Exportar cookies",
        "Signing in with a Google account and using it to play audio can get that account banned by Google. Use an account you can afford to lose, and prefer a secondary one." => {
            "Iniciar sesión con una cuenta de Google y usarla para reproducir audio puede hacer que Google la banee. Usa una cuenta que puedas permitirte perder y prefiere una secundaria."
        }
        "Paste cookies (must include SAPISID) or a cookies.txt export" => {
            "Pega las cookies (deben incluir SAPISID) o un cookies.txt"
        }
        "This session cannot open private playlists. Use the session from a browser:" => {
            "Esta sesión no puede abrir playlists privadas. Usa la sesión de un navegador:"
        }
        "YouTube session imported" => "Sesión de YouTube importada",
        "The YouTube session expired. Import it again." => {
            "La sesión de YouTube expiró. Impórtala de nuevo."
        }
        "Those cookies have no SAPISID" => "Esas cookies no incluyen SAPISID",
        "That session only signs requests; keeping the one that opens your lists" => {
            "Esa sesión solo firma peticiones; se conserva la que abre tus listas"
        }
        "There is no YouTube session to export" => "No hay una sesión de YouTube que exportar",
        "Free cached covers after the app has been idle for a while to reclaim memory. Playback is not affected." => {
            "Libera las carátulas en caché cuando la app lleva un rato sin usarse. No afecta la reproducción."
        }
        "Relay server that syncs playback between devices. Rooms and their passwords are set from the Rooms panel." => {
            "Servidor relay que sincroniza la reproducción entre dispositivos. Las salas y sus contraseñas se configuran desde el panel de Salas."
        }
        "Relay server" => "Servidor relay",
        "Auth token" => "Token de autenticación",
        "Pinned by RELAY_API_URL, so changes are not saved." => {
            "Fijado por RELAY_API_URL, los cambios no se guardan."
        }
        "Checking…" => "Comprobando…",
        "Connected" => "Conectado",
        "Not connected" => "No conectado",
        "Version" => "Versión",
        "View release" => "Ver versión",
        "Download" => "Descargar",
        "A minimal, native Jota. GPL-3.0." => "Un Jota mínimo y nativo. GPL-3.0.",
        "Back up every playlist, Spotify and YouTube included, in a JSON file or restore them." => {
            "Copia todas tus playlists, también las de Spotify y YouTube, en un archivo JSON o restáuralas."
        }
        "Export backup" => "Exportar copia",
        "Import backup" => "Importar copia",
        "Back up every playlist, Spotify and YouTube included, in a JSON file or restore them. Re-importing keeps the newer copy when a playlist already exists." => {
            "Copia todas tus playlists, también las de Spotify y YouTube, en un archivo JSON o restáuralas. Al reimportar se conserva la copia más reciente si la playlist ya existe."
        }
        "Autosave" => "Autoguardado",
        "Save your library to jota.backup.json every so often, in a folder or an account like Google Drive or Dropbox." => {
            "Guarda tu biblioteca en jota.backup.json cada cierto tiempo, en una carpeta o una cuenta como Google Drive o Dropbox."
        }
        "Location" => "Ubicación",
        "No location selected" => "Sin ubicación seleccionada",
        "Choose location" => "Elegir ubicación",
        "Change location" => "Cambiar ubicación",
        "Last save" => "Último guardado",
        "Size" => "Tamaño",
        "Playlists in the file" => "Playlists en el fichero",
        "Imported" => "Importadas",
        "Autosave automatically" => "Autoguardado automático",
        "Autoimport" => "Autoimportar",
        "Bring to your library what other devices have saved in the file too." => {
            "Trae también a tu biblioteca lo que otros dispositivos hayan guardado en el fichero."
        }
        "Every" => "Cada",
        "Save now" => "Guardar ahora",
        "Reload backup" => "Recargar backup",
        "Overwrite the file" => "Sobrescribir el fichero",
        "The file carries your relay address and its token in plain text, so anyone who can open it can use them." => {
            "El fichero lleva la dirección de tu relay y su token en claro, así que cualquiera que pueda abrirlo puede usarlos."
        }
        "Session encryption" => "Cifrado de sesiones",
        "Same pass on every device" => "La misma contraseña en todos los dispositivos",
        "When set, the YouTube and Spotify sessions are encrypted and synced through the backup. Leave empty to keep them local only." => {
            "Con contraseña, las sesiones de YouTube y Spotify se cifran y viajan en el backup. Déjala vacía para mantenerlas solo en este dispositivo."
        }
        "The last save failed. Check the location and its permissions." => {
            "El último guardado falló. Revisa la ubicación y sus permisos."
        }
        "never" => "nunca",
        "KB" => "KB",
        "Import" => "Importar",
        "Interval" => "Intervalo",
        "Set a backup location first" => "Configura primero una ubicación de copia",
        "Backup saved" => "Copia guardada",
        "Backup reloaded" => "Copia recargada",
        "Logs saved to {}" => "Registros guardados en {}",
        "Cookies saved to {}" => "Cookies guardadas en {}",
        "Pin window" => "Fijar ventana",
        "Unpin window" => "Desfijar ventana",
        "This session does not let a window stay on top." => {
            "Esta sesión no permite que una ventana permanezca encima."
        }
        "Backup exported to {}" => "Copia exportada a {}",
        "Backup restored · {} changes" => "Copia restaurada · {} cambios",
        "No backup in {}" => "No hay copia en {}",
        "No folder to save the backup" => "No hay carpeta donde guardar la copia",
        "No folder to save the cookies" => "No hay carpeta donde guardar las cookies",
        "Play" => "Reproducir",
        "Pause" => "Pausar",
        "Queue" => "Cola",
        "Search" => "Buscar",
        "Cancel" => "Cancelar",
        "Create" => "Crear",
        "Delete" => "Eliminar",
        "Rename" => "Renombrar",
        "Save" => "Guardar",
        "Close" => "Cerrar",
        "Back" => "Atrás",
        "Forward" => "Adelante",
        "Playlists" => "Playlists",
        "Artists" => "Artistas",
        "Following" => "Siguiendo",
        "Loading" => "Cargando",
        "Nothing playing" => "Nada sonando",
        "Add to playlist" => "Añadir a playlist",
        "Add to queue" => "Añadir a la cola",
        "Select" => "Seleccionar",
        "Deselect" => "Deseleccionar",
        "Follow playlist" => "Seguir playlist",
        "Unfollow playlist" => "Dejar de seguir playlist",
        "Share" => "Compartir",
        "Share Spotify" => "Compartir Spotify",
        "Share YouTube" => "Compartir YouTube",
        "Link copied." => "Enlace copiado.",
        "More actions" => "Más acciones",
        "Jump to current song" => "Ir a la canción actual",
        "Assign audio" => "Asignar audio",
        "Below current" => "Debajo del actual",
        "Move down" => "Bajar",
        "Remove" => "Quitar",
        "Remove from playlist" => "Quitar de la playlist",
        "Are you sure?" => "¿Seguro?",
        "New playlist" => "Nueva playlist",
        "Playlist name" => "Nombre de la playlist",
        "Rename playlist" => "Renombrar playlist",
        "Delete playlist" => "Eliminar playlist",
        "Delete “{name}”? This cannot be undone." => {
            "¿Eliminar «{name}»? No se puede deshacer."
        }
        "Sort by" => "Ordenar por",
        "YouTube audio" => "Audio de YouTube",
        "Paste the video ID or URL." => "Pega el ID o la URL del vídeo.",
        "Search YouTube and pick the right video." => "Busca en YouTube y elige el vídeo correcto.",
        "ID or URL" => "ID o URL",
        "Song, artist or link..." => "Canción, artista o enlace...",
        "Type to search YouTube." => "Escribe para buscar en YouTube.",
        "Could not search YouTube." => "No se pudo buscar en YouTube.",
        "Paste a valid YouTube video ID or URL." => {
            "Pega un ID o una URL válida de vídeo de YouTube."
        }
        "Assign audio by ID or URL" => "Asignar audio por ID o URL",
        "Search YouTube" => "Buscar en YouTube",
        "Stop preview" => "Detener previsualización",
        "Change audio" => "Cambiar audio",
        "Clear" => "Limpiar",
        "Could not play this video." => "No se pudo reproducir este vídeo.",
        "this playlist" => "esta playlist",
        "{} songs" => "{} canciones",
        "{count} songs" => "{count} canciones",
        "1 follower" => "1 seguidor",
        "1 selected" => "1 seleccionado",
        "1 track" => "1 canción",
        "{count} tracks" => "{count} canciones",
        "No songs" => "Sin canciones",
        "Liked Songs" => "Canciones que te gustan",
        "No results" => "Sin resultados",
        "Unavailable" => "No disponible",
        "No videos found." => "No se encontraron vídeos.",
        "No channels found." => "No se encontraron canales.",
        "No playlists found." => "No se encontraron playlists.",
        "Albums" => "Álbumes",
        "Singles" => "Sencillos",
        "Compilations" => "Recopilatorios",
        "Appears on" => "Aparece en",
        "Top tracks" => "Más escuchadas",
        "All" => "Todas",
        "No playlists yet" => "Aún no hay listas",
        "No local playlists yet" => "Aún no hay playlists locales",
        "No Spotify playlists." => "No hay playlists de Spotify.",
        "Connect with Spotify" => "Conectar con Spotify",
        "Connect with Spotify to use this." => "Conecta con Spotify para usar esto.",
        "Tap Connect with Spotify and log in with your account." => {
            "Pulsa Conectar con Spotify e inicia sesión con tu cuenta."
        }
        "Return to Jondo when the login finishes." => {
            "Vuelve a Jondo cuando termine el inicio de sesión."
        }
        "Your playlists and followed profiles appear on their own." => {
            "Tus listas y perfiles seguidos aparecen solos."
        }
        "Spotify is unreachable" => "Spotify no responde",
        "Risk: Spotify may block an account that uses Jondo." => {
            "Riesgo: Spotify puede bloquear una cuenta que use Jondo."
        }
        "This needs a live connection. Your saved library still works." => {
            "Esto necesita conexión. Tu biblioteca guardada sigue funcionando."
        }
        "Retry" => "Reintentar",
        "Connect Spotify, follow a YouTube playlist, or create a local one — everything shows up here." => {
            "Conecta Spotify, sigue una lista de YouTube o crea una local: todo aparecerá aquí."
        }
        "Follow a YouTube playlist or create a local one — both show up here." => {
            "Sigue una lista de YouTube o crea una local: ambas aparecerán aquí."
        }
        "Create a local playlist to collect songs that live on this device." => {
            "Crea una playlist local para reunir canciones guardadas en este dispositivo."
        }
        "In the search bar at the top, search a playlist by name or paste its link." => {
            "En la barra de búsqueda de arriba, busca una lista por nombre o pega su enlace."
        }
        "Open the Playlists tab in the results and tap the heart to follow it." => {
            "Abre la pestaña Listas en los resultados y toca el corazón para seguirla."
        }
        "Or use the + button to create a local playlist." => {
            "O usa el botón + para crear una lista local."
        }
        "Tap the + button in the top corner of your playlists." => {
            "Toca el botón + en la esquina superior de tus playlists."
        }
        "Name it and it appears in this list." => "Ponle nombre y aparecerá en esta lista.",
        "Open it and add songs from any source." => "Ábrela y añade canciones de cualquier fuente.",
        "Nothing to see yet" => "Aún no sigues a nadie",
        "Spotify profiles and YouTube channels you follow appear on this page." => {
            "Los perfiles de Spotify y canales de YouTube que sigas aparecerán en esta página."
        }
        "Follow a Spotify profile or a YouTube channel and it will show up here." => {
            "Sigue un perfil de Spotify o un canal de YouTube y aparecerá aquí."
        }
        "In the search bar at the top, search an artist, profile or channel." => {
            "En la barra de búsqueda de arriba, busca un artista, perfil o canal."
        }
        "Open the profile or channel and tap the heart to follow it." => {
            "Abre el perfil o canal y toca el corazón para seguirlo."
        }
        "No YouTube channels followed yet." => "Aún no sigues ningún canal de YouTube.",
        "No Spotify profiles followed yet." => "Aún no sigues ningún perfil de Spotify.",
        "Follow" => "Seguir",
        "Unfollow" => "Dejar de seguir",
        "Oldest" => "Más antiguas",
        "Newest" => "Más recientes",
        "Shortest" => "Más cortas",
        "Longest" => "Más largas",
        "Track" => "Canción",
        "User" => "Usuario",
        "Album" => "Álbum",
        "Playlist" => "Playlist",
        "Artist" => "Artista",
        "Added to library" => "Añadida a la biblioteca",
        "Added to playlist" => "Añadida a la playlist",
        "Song already in playlist" => "La canción ya está en la playlist",
        "Removed from library" => "Eliminada de la biblioteca",
        "Saved to Rooms" => "Guardada en Salas",
        "Saved" => "Guardado",
        "Checking" => "Comprobando",
        "Offline" => "Sin conexión",
        "Connect" => "Conectar",
        "Connecting" => "Conectando",
        "Connecting..." => "Conectando...",
        "Joining..." => "Uniéndose...",
        "Generate" => "Generar",
        "Optional" => "Opcional",
        "Relay token required" => "Token del relay requerido",
        "This relay requires an auth token, so rooms stay hidden until you add one." => {
            "Este relay requiere un token de autenticación, así que las salas permanecen ocultas hasta que lo añadas."
        }
        "Enter the relay auth token." => "Introduce el token de autenticación del relay.",
        "Room code" => "Código de sala",
        "Room password" => "Contraseña de sala",
        "Save room" => "Guardar sala",
        "Stop session" => "Detener sesión",
        "Switch room" => "Cambiar de sala",
        "Leave room" => "Salir de la sala",
        "Leave session" => "Salir de la sesión",
        "Join or host a room" => "Únete o crea una sala",
        "No relay server yet" => "Aún no hay servidor relay",
        "No saved rooms. Connect to a room and save it from the Rooms panel." => {
            "No hay salas guardadas. Conéctate a una sala y guárdala desde el panel de Salas."
        }
        "Rooms are hosted by a relay, so you need one before you can join or host a room." => {
            "Las salas se alojan en un relay, así que necesitas uno para unirte o crear una sala."
        }
        "Set the relay server address." => "Indica la dirección del servidor relay.",
        "The host left" => "El anfitrión se fue",
        "Come back here and join or host a room." => "Vuelve aquí y únete o crea una sala.",
        "Open Settings and go to the Rooms section." => "Abre Ajustes y ve a la sección de Salas.",
        "Open settings" => "Abrir ajustes",
        "Relay: {url}" => "Relay: {url}",
        "If nobody is hosting it yet, you become the host; otherwise you join and listen in sync." => {
            "Si nadie la está alojando aún, te conviertes en el anfitrión; si no, te unes y escuchas en sincronía."
        }
        "Waiting for the room to load..." => "Esperando a que cargue la sala...",
        "You are in room {}. Edit the code to switch rooms." => {
            "Estás en la sala {}. Edita el código para cambiar de sala."
        }
        "{} connected" => "{} conectados",
        "Deselect all" => "Deseleccionar todo",
        "Select all" => "Seleccionar todo",
        "Search in YouTube" => "Buscar en YouTube",
        "Search type" => "Tipo de búsqueda",
        "Search in YouTube (videos, playlists or channels)" => {
            "Buscar en YouTube (vídeos, playlists o canales)"
        }
        "Song, artist, or Spotify link/ID..." => "Canción, artista, o enlace/ID de Spotify...",
        "Spotify username or link..." => "Usuario o enlace de Spotify...",
        "Paste a Spotify {} link, URI or ID..." => {
            "Pega un enlace, URI o ID de Spotify de tipo {}..."
        }
        "Spotify only opens a {} from a link, URI or ID. Paste one, or search by name under another type." => {
            "Spotify solo abre un {} desde un enlace, URI o ID. Pega uno o busca por nombre en otro tipo."
        }
        "Nothing to see" => "Nada que ver",
        "No videos" => "Sin vídeos",
        "Cover" => "Carátula",
        "Duration" => "Duración",
        "Add" => "Añadir",
        "Backup location saved" => "Ubicación de la copia guardada",
        "Backup merged · {} changes" => "Copia fusionada · {} cambios",
        "Backup reloaded · {} changes" => "Copia recargada · {} cambios",
        "Cannot read the backup file" => "No se puede leer el archivo de copia",
        "Cannot read the backup location" => "No se puede leer la ubicación de la copia",
        "Cannot write the backup file" => "No se puede escribir el archivo de copia",
        "Compact player" => "Reproductor compacto",
        "Complete the sign-in in your browser" => "Completa el inicio de sesión en tu navegador",
        "Connect Spotify to search its catalog" => "Conecta Spotify para buscar en su catálogo",
        "Create in the cloud" => "Crear en la nube",
        "Discord" => "Discord",
        "Error" => "Error",
        "Expand" => "Expandir",
        "GitHub" => "GitHub",
        "Grid view" => "Vista de cuadrícula",
        "Info" => "Info",
        "List view" => "Vista de lista",
        "Loading playlist" => "Cargando playlist",
        "Locked" => "Bloqueado",
        "Maximize" => "Maximizar",
        "Minimize" => "Minimizar",
        "More" => "Más",
        "Move below current" => "Mover debajo del actual",
        "Music" => "Música",
        "Mute" => "Silenciar",
        "Next" => "Siguiente",
        "No YouTube match" => "Sin coincidencia en YouTube",
        "No place to stage the backup" => "No hay dónde preparar la copia",
        "Open from the cloud" => "Abrir en la nube",
        "Open release" => "Abrir versión",
        "Phone folder" => "Carpeta del teléfono",
        "Pin" => "Fijar",
        "Previous" => "Anterior",
        "Refresh" => "Actualizar",
        "Reload" => "Recargar",
        "Reorder" => "Reordenar",
        "Repeat" => "Repetir",
        "Selected" => "Seleccionado",
        "Shuffle" => "Aleatorio",
        "Sign in to Spotify again" => "Inicia sesión en Spotify de nuevo",
        "Sort" => "Ordenar",
        "The backup call failed" => "La llamada de copia falló",
        "This relay requires a token." => "Este relay requiere un token.",
        "This relay requires an auth token." => "Este relay requiere un token de autenticación.",
        "Could not reach the relay. Check the address and your connection." => {
            "No se pudo contactar con el relay. Revisa la dirección y tu conexión."
        }
        "The relay answered {}." => "El relay respondió {}.",
        "The relay accepted this token." => "El relay aceptó este token.",
        "The relay rejected this token." => "El relay rechazó este token.",
        "Volume" => "Volumen",
        "Warning" => "Aviso",
        "playlists" => "playlists",
        "Backup exported" => "Copia exportada",
        "Browser" => "Navegador",
        "Google Drive" => "Google Drive",
        "Phone" => "Teléfono",
        "Use the {} session" => "Usar la sesión de {}",
        "Logs shared" => "Registros compartidos",
        "Cookies shared" => "Cookies compartidas",
        "Updating…" => "Actualizando…",
        "Update" => "Actualizar",
        "Update to" => "Actualizar a",
        "Jondo {version} is available" => "Jondo {version} está disponible",
        "Confirm the install on your phone." => "Confirma la instalación en tu teléfono.",
        "No folder to save the update" => "No hay carpeta donde guardar la actualización",
        "The session pass does not match that file" => {
            "La contraseña de sesión no coincide con la de ese archivo"
        }
        "The last save could not write: the service served a cached copy of the file. Choose the location again." => {
            "El último guardado no pudo escribir: el servicio sirvió una copia en caché del archivo. Elige la ubicación de nuevo."
        }
        "The file carries a session this device cannot open. Set the pass it was encrypted with to bring it here." => {
            "El archivo lleva una sesión que este dispositivo no puede abrir. Pon la contraseña con la que se cifró para traerla aquí."
        }
        "Pick the jota.backup.json you already have in Google Drive or Dropbox." => {
            "Elige el jota.backup.json que ya tienes en Google Drive o Dropbox."
        }
        "Creates the file in the cloud. If it already existed, the provider makes a duplicate." => {
            "Crea el archivo en la nube. Si ya existía, el proveedor hace un duplicado."
        }
        "Where to save?" => "¿Dónde guardar?",
        "A local folder, such as the one Syncthing syncs." => {
            "Una carpeta local, como la que sincroniza Syncthing."
        }
        "The provider created a new file instead of using the one that already existed. There are two backups now: pick the right one." => {
            "El proveedor creó un archivo nuevo en vez de usar el que ya existía. Ahora hay dos copias: elige la correcta."
        }
        "Choose the sync folder" => "Elige la carpeta de sincronización",
        "In-app sign-in is not available on this platform. Paste your cookies or import a cookies.txt export below." => {
            "El inicio de sesión integrado no está disponible en esta plataforma. Pega tus cookies o importa un cookies.txt abajo."
        }
        "Paste cookies or import a cookies.txt" => "Pegar cookies o importar un cookies.txt",
        "Import a cookies.txt exported while signed in: on a youtube.com tab use an extension (Get cookies.txt), or run copy(document.cookie) only if you do not need private playlists. The cookies that open your lists (__Secure-3PSID) are HttpOnly and are not in document.cookie." => {
            "Importa un cookies.txt exportado con la sesión iniciada: en una pestaña de youtube.com usa una extensión (Get cookies.txt) o ejecuta copy(document.cookie) solo si no necesitas playlists privadas. Las cookies que abren tus listas (__Secure-3PSID) son HttpOnly y no están en document.cookie."
        }
        "Reading the cookies of {}…" => "Leyendo las cookies de {}…",
        "Reads the YouTube session {} already holds on this machine. Close it first if the read fails." => {
            "Lee la sesión de YouTube que {} ya tiene en este equipo. Ciérralo primero si la lectura falla."
        }
        "Could not use the {} session" => "No se pudo usar la sesión de {}",
        "Import cookies" => "Importar cookies",
        "Cannot read that file: {}" => "No se puede leer ese archivo: {}",
        "Risk" => "Riesgo",
        "Import file" => "Importar fichero",
        "Go to this playlist" => "Ir a esta lista",
        "Go to this album" => "Ir a este álbum",
        "Go to this artist" => "Ir a este artista",
        "Go to this channel" => "Ir a este canal",
        "Listen to this song" => "Escuchar canción",
        "Dismiss" => "Descartar",
        "Channel" => "Canal",
        "Connect Spotify to play this song" => "Conecta Spotify para reproducir esta canción",
        _ => key,
    }
}
