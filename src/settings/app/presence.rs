use crate::shared::discord::Activity;
use crate::shell::app::Jondo;

impl Jondo {
    pub(crate) fn tick_presence(&mut self, cx: &mut gpui::Context<Self>) {
        // Android has no Discord client to reach and no socket to speak to.
        if cfg!(target_os = "android") {
            self.clear_presence(cx);
            return;
        }

        if !self.store.discord {
            self.clear_presence(cx);
            return;
        }

        let (id, playing) = {
            let player = self.player.read(cx);
            let Some(song) = player.current() else {
                self.clear_presence(cx);
                return;
            };
            (song.id.clone(), player.playing)
        };

        if self.settings.read(cx).discord_key.as_ref() == Some(&(id, playing)) {
            return;
        }

        let (song, progress, duration) = {
            let player = self.player.read(cx);
            let Some(song) = player.current() else {
                return;
            };
            (song.clone(), player.progress, player.duration)
        };

        let activity = Activity {
            details: song.name.clone(),
            state: song.artists.join(", "),
            album: song.album.clone(),
            artwork: song.cover.clone().unwrap_or_default(),
            playing,
            position: progress,
            duration,
        };
        self.settings
            .update(cx, |settings, _| settings.discord.set(activity));
        self.settings.update(cx, |settings, _| {
            settings.discord_key = Some((song.id.clone(), playing))
        });
        self.settings
            .update(cx, |settings, _| settings.discord_on = true);
    }

    pub(super) fn clear_presence(&mut self, cx: &mut gpui::Context<Self>) {
        if self.settings.read(cx).discord_on {
            self.settings
                .update(cx, |settings, _| settings.discord.clear());
            self.settings
                .update(cx, |settings, _| settings.discord_on = false);
        }
    }
}
