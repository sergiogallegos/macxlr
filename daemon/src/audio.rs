use crate::{OVERRIDE_SAMPLER_INPUT, OVERRIDE_SAMPLER_OUTPUT};
use anyhow::{Result, anyhow, bail};
use enum_map::EnumMap;
use fancy_regex::Regex;
use goxlr_audio::player::{Player, PlayerState};
use goxlr_audio::recorder::BufferedRecorder;
use goxlr_audio::recorder::RecorderState;
use goxlr_audio::{AtomicF64, get_audio_inputs};
use goxlr_types::SampleBank;
use goxlr_types::SampleButtons;
use log::{debug, error, info, warn};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;

#[derive(Debug)]
pub struct AudioHandler {
    output_device: Option<String>,

    buffered_input: Option<Arc<BufferedRecorder>>,
    recorder_buffer: u16,

    last_device_check: Option<Instant>,
    active_streams: EnumMap<SampleBank, EnumMap<SampleButtons, Option<StateManager>>>,

    process_task: Option<ProcessTask>,
}

pub struct AudioFile {
    pub(crate) file: PathBuf,
    pub(crate) name: String,
    pub(crate) gain: Option<f64>,
    pub(crate) start_pct: Option<f64>,
    pub(crate) stop_pct: Option<f64>,
    pub(crate) fade_on_stop: Option<u32>,
}

#[derive(Debug)]
pub struct ProcessTask {
    bank: SampleBank,
    button: SampleButtons,
    file: PathBuf,

    player: AudioPlaybackState,
}

#[derive(Debug)]
struct AudioPlaybackState {
    handle: Option<JoinHandle<()>>,
    state: PlayerState,
}

#[derive(Debug)]
struct AudioRecordingState {
    file: PathBuf,
    handle: Option<JoinHandle<()>>,
    state: RecorderState,
}

#[derive(Debug)]
struct StateManager {
    pub(crate) stream_type: StreamType,
    pub(crate) recording: Option<AudioRecordingState>,
    pub(crate) playback: Option<AudioPlaybackState>,
}

#[derive(Debug, PartialEq)]
enum StreamType {
    Playback,
    Recording,
}

// I could probably use a trait for this..
impl AudioPlaybackState {
    pub fn wait(&mut self) {
        let _ = self.handle.take().map(JoinHandle::join);
    }

    pub fn is_finished(&self) -> bool {
        if let Some(handle) = &self.handle {
            return handle.is_finished();
        }
        true
    }
}

impl AudioRecordingState {
    pub fn wait(&mut self) {
        let _ = self.handle.take().map(JoinHandle::join);
    }

    pub fn is_finished(&self) -> bool {
        if let Some(handle) = &self.handle {
            return handle.is_finished();
        }
        true
    }
}

impl AudioHandler {
    fn compile_patterns(patterns: &[&str], context: &str) -> Vec<Regex> {
        patterns
            .iter()
            .filter_map(|pattern| match Regex::new(pattern) {
                Ok(regex) => Some(regex),
                Err(error) => {
                    error!("Invalid regex in {}: {} ({})", context, pattern, error);
                    None
                }
            })
            .collect()
    }

    pub fn new(recorder_buffer: u16) -> Result<Self> {
        // Find the Input Device..
        let mut handler = Self {
            output_device: None,

            buffered_input: None,
            recorder_buffer,

            last_device_check: None,
            active_streams: EnumMap::default(),

            process_task: None,
        };

        // Immediately initialise the recorder, and let it try to handle stuff.
        let recorder = BufferedRecorder::new(
            handler.get_input_device_string_patterns(),
            recorder_buffer as usize,
        )?;
        let arc_recorder = Arc::new(recorder);
        let inner_recorder = arc_recorder.clone();
        handler.buffered_input.replace(arc_recorder);

        // Fire off the new thread to listen to audio..
        thread::spawn(move || inner_recorder.listen());

        // Attempt to do an early-find on the output device, if this fails we'll try again
        // when a sample is played back.
        handler.find_device(true);
        Ok(handler)
    }

    pub fn update_record_buffer(&mut self, recorder_buffer: u16) -> Result<()> {
        self.recorder_buffer = recorder_buffer;

        if let Some(recorder) = &self.buffered_input {
            recorder.stop();
        }

        let recorder = BufferedRecorder::new(
            self.get_input_device_string_patterns(),
            recorder_buffer as usize,
        )?;
        let arc_recorder = Arc::new(recorder);
        let inner_recorder = arc_recorder.clone();

        // This should force a STOP of any pre-existing recorders...
        self.buffered_input.replace(arc_recorder);

        // Fire off the new thread to listen to audio..
        thread::spawn(move || inner_recorder.listen());
        Ok(())
    }

    fn refresh_input_recorder(&mut self) -> Result<()> {
        self.update_record_buffer(self.recorder_buffer)
    }

    fn get_output_device_patterns(&self) -> Vec<Regex> {
        let override_output = OVERRIDE_SAMPLER_OUTPUT
            .lock()
            .map(|value| value.deref().clone())
            .unwrap_or_else(|error| {
                warn!("Unable to read sampler output override: {}", error);
                None
            });
        if let Some(device) = override_output {
            return Self::compile_patterns(&[device.as_str()], "sampler output override");
        }

        Self::compile_patterns(
            &[
                // Linux
                "goxlr_sample",
                "GoXLR_0_8_9",
                "GoXLR.*HiFi__Line3__sink",
                // MacOS
                "CoreAudio\\*Sample(?:(?!Mini).)*$",
                "CoreAudio\\*Sampler(?:(?!Mini).)*$",
                "CoreAudio\\*GoXLR(?:(?!Mini).)*$",
                // Windows
                "^WASAPI\\*Sample(?:(?!Mini).)*$",
            ],
            "sampler output patterns",
        )
    }

    fn get_input_device_patterns(&self) -> Vec<Regex> {
        let override_input = OVERRIDE_SAMPLER_INPUT
            .lock()
            .map(|value| value.deref().clone())
            .unwrap_or_else(|error| {
                warn!("Unable to read sampler input override: {}", error);
                None
            });
        if let Some(device) = override_input {
            return Self::compile_patterns(&[device.as_str()], "sampler input override");
        }

        Self::compile_patterns(
            &[
                // Linux
                "goxlr_sample.*source",
                "GoXLR_0_4_5.*source",
                "GoXLR.*HiFi__Line5__source",
                // MacOS
                "CoreAudio\\*Sampler(?:(?!Mini).)*$",
                "CoreAudio\\*Sample(?:(?!Mini).)*$",
                "CoreAudio\\*GoXLR(?:(?!Mini).)*$",
                // Windows
                "^WASAPI\\*Sample(?:(?!Mini).)*$",
            ],
            "sampler input patterns",
        )
    }

    fn get_input_device_string_patterns(&self) -> Vec<String> {
        let override_input = OVERRIDE_SAMPLER_INPUT
            .lock()
            .map(|value| value.deref().clone())
            .unwrap_or_else(|error| {
                warn!("Unable to read sampler input override: {}", error);
                None
            });
        if let Some(device) = override_input {
            return vec![device];
        }

        let patterns = vec![
            // Linux
            String::from("goxlr_sample.*source"),
            String::from("GoXLR_0_4_5.*source"),
            String::from("GoXLR.*HiFi__Line5__source"),
            // MacOS
            String::from("CoreAudio\\*Sampler(?:(?!Mini).)*$"),
            String::from("CoreAudio\\*Sample(?:(?!Mini).)*$"),
            String::from("CoreAudio\\*GoXLR(?:(?!Mini).)*$"),
            // Windows
            String::from("^WASAPI\\*Sample(?:(?!Mini).)*$"),
        ];

        patterns
    }

    fn find_device(&mut self, is_output: bool) {
        debug!("Attempting to Find Device..");
        if let Some(last_check) = self.last_device_check
            && last_check + Duration::from_secs(5) > Instant::now()
        {
            return;
        }

        let device_list = match is_output {
            true => goxlr_audio::get_audio_outputs(),
            false => get_audio_inputs(),
        };

        let pattern_matchers = match is_output {
            true => self.get_output_device_patterns(),
            false => self.get_input_device_patterns(),
        };

        debug!(
            "Looking for {} device using patterns: {:?}",
            if is_output { "output" } else { "input" },
            pattern_matchers
                .iter()
                .map(|pattern| pattern.as_str())
                .collect::<Vec<_>>()
        );

        let device = device_list
            .iter()
            .find(|output| {
                pattern_matchers.iter().any(|pattern| {
                    if let Ok(result) = pattern.is_match(output) {
                        return result;
                    }
                    false
                })
            })
            .cloned();

        if let Some(device) = &device {
            debug!("Found Device: {}", device);
        } else {
            warn!("Audio Device Not Found, Available Devices:");
            device_list.iter().for_each(|name| info!("{}", name));
        }

        self.last_device_check = Some(Instant::now());

        if is_output {
            self.output_device = device;
        }
    }

    pub async fn check_playing(&mut self) -> bool {
        let mut state_changed = false;

        // Iterate over the Sampler Banks..
        for bank in SampleBank::iter() {
            // Iterate over the buttons..
            for button in SampleButtons::iter() {
                if let Some(state) = &self.active_streams[bank][button] {
                    if state.stream_type == StreamType::Recording {
                        if let Some(recording) = &state.recording
                            && recording.is_finished()
                        {
                            self.active_streams[bank][button] = None;
                            state_changed = true;
                        }
                    } else if let Some(playback) = &state.playback
                        && playback.is_finished()
                    {
                        self.active_streams[bank][button] = None;
                        state_changed = true;
                    }
                }
            }
        }

        state_changed
    }

    pub fn is_sample_playing(&self, bank: SampleBank, button: SampleButtons) -> bool {
        if let Some(stream) = &self.active_streams[bank][button]
            && stream.playback.is_some()
        {
            return true;
        }
        false
    }

    pub fn get_playing_file(&self, bank: SampleBank, button: SampleButtons) -> Option<PathBuf> {
        if let Some(stream) = &self.active_streams[bank][button]
            && let Some(manager) = &stream.playback
        {
            return Some(manager.state.playing_file.clone());
        }
        None
    }

    pub fn sample_recording(&self, bank: SampleBank, button: SampleButtons) -> bool {
        if let Some(stream) = &self.active_streams[bank][button]
            && stream.recording.is_some()
        {
            return true;
        }
        false
    }

    pub fn is_sample_recording(&self) -> bool {
        for bank in SampleBank::iter() {
            for button in SampleButtons::iter() {
                if let Some(manager) = &self.active_streams[bank][button]
                    && manager.recording.is_some()
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_sample_stopping(&self, bank: SampleBank, button: SampleButtons) -> bool {
        if let Some(state) = &self.active_streams[bank][button] {
            if state.stream_type == StreamType::Recording {
                return false;
            }

            if let Some(player) = &state.playback {
                return player.state.stopping.load(Ordering::Relaxed);
            }
        }

        false
    }

    pub async fn play_for_button(
        &mut self,
        bank: SampleBank,
        button: SampleButtons,
        audio: AudioFile,
        loop_track: bool,
    ) -> Result<()> {
        if self.output_device.is_none() {
            self.find_device(true);
        }

        let build_player = |output_device: &str| {
            Player::new(
                &audio.file,
                Some(output_device.to_string()),
                audio.fade_on_stop,
                audio.start_pct,
                audio.stop_pct,
                audio.gain,
            )
        };

        let mut player = if let Some(output_device) = &self.output_device {
            match build_player(output_device) {
                Ok(player) => player,
                Err(error) => {
                    warn!(
                        "Unable to create player for cached sampler output '{}': {}. Retrying device lookup.",
                        output_device, error
                    );

                    self.output_device = None;
                    self.last_device_check = None;
                    self.find_device(true);

                    let Some(refreshed_output) = &self.output_device else {
                        return Err(anyhow!(
                            "Unable to play Sample, Output device not found after retry"
                        ));
                    };

                    build_player(refreshed_output)?
                }
            }
        } else {
            return Err(anyhow!("Unable to play Sample, Output device not found"));
        };

        let state = player.get_state();
        let handler = thread::spawn(move || {
            if !loop_track {
                let result = player.play();
                if let Err(error) = result {
                    warn!("Playback Error: {}", error);
                }
            } else {
                let result = player.play_loop();
                if let Err(error) = result {
                    warn!("Loop Playback Error: {}", error);
                }
            }
        });

        self.active_streams[bank][button] = Some(StateManager {
            stream_type: StreamType::Playback,
            recording: None,
            playback: Some(AudioPlaybackState {
                handle: Some(handler),
                state,
            }),
        });

        Ok(())
    }

    pub async fn restart_for_button(
        &mut self,
        bank: SampleBank,
        button: SampleButtons,
    ) -> Result<()> {
        if let Some(player) = &mut self.active_streams[bank][button] {
            if player.stream_type == StreamType::Recording {
                return Err(anyhow!(
                    "Attempted to Restart Playback on Recording Stream.."
                ));
            }

            if let Some(playback_state) = &mut player.playback {
                // We'll set the value to true, which will be a signal to the
                // audio player to restart the track, once that signal is processed
                // it'll be reset back to false.
                playback_state
                    .state
                    .restart_track
                    .store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    pub async fn stop_playback(
        &mut self,
        bank: SampleBank,
        button: SampleButtons,
        force: bool,
    ) -> Result<()> {
        if let Some(player) = &mut self.active_streams[bank][button] {
            if player.stream_type == StreamType::Recording {
                // TODO: We can proably use this..
                return Err(anyhow!("Attempted to Stop Playback on Recording Stream.."));
            }

            if let Some(playback_state) = &mut player.playback {
                if playback_state.state.stopping.load(Ordering::Relaxed) {
                    // We should be stopping already, force the shutdown.
                    debug!("Forcing Stop of Audio on {} {}..", bank, button);
                    playback_state
                        .state
                        .force_stop
                        .store(true, Ordering::Relaxed);

                    // We'll wait for this thread to complete before proceeding..
                    playback_state.wait();
                    self.active_streams[bank][button] = None;
                    return Ok(());
                }

                // TODO: Tidy this!
                if force {
                    playback_state
                        .state
                        .force_stop
                        .store(true, Ordering::Relaxed);
                }

                // We're not currently in a stopping state, trigger it.
                playback_state.state.stopping.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    pub fn record_for_button(
        &mut self,
        path: PathBuf,
        bank: SampleBank,
        button: SampleButtons,
    ) -> Result<()> {
        let recorder = if let Some(recorder) = &self.buffered_input {
            if recorder.is_ready() {
                recorder.clone()
            } else {
                warn!("Sampler recorder is not ready, refreshing input device detection.");
                self.refresh_input_recorder()?;

                let Some(recorder) = &self.buffered_input else {
                    bail!("No valid Input Device was Found");
                };

                if !recorder.is_ready() {
                    warn!("Sampler not ready, possibly missing Sample device. Not recording.");
                    debug!("Available Audio Inputs: ");
                    get_audio_inputs()
                        .iter()
                        .for_each(|name| debug!("{}", name));

                    bail!("Sampler is not ready to handle recording (possibly missing device?)");
                }

                recorder.clone()
            }
        } else {
            self.refresh_input_recorder()?;
            self.buffered_input
                .as_ref()
                .cloned()
                .ok_or_else(|| anyhow!("No valid Input Device was Found"))?
        };

        let state = RecorderState {
            stop: Arc::new(AtomicBool::new(false)),
            gain: Arc::new(AtomicF64::new(1.)),
        };

        let inner_recorder = recorder;
        let inner_path = path.clone();
        let inner_state = state.clone();

        let handler = thread::spawn(move || {
            let result = inner_recorder.record(&inner_path, inner_state);
            if let Err(error) = result {
                error!("Recording Error: {}", error);
            }
        });

        self.active_streams[bank][button] = Some(StateManager {
            stream_type: StreamType::Recording,
            recording: Some(AudioRecordingState {
                file: path,
                handle: Some(handler),
                state,
            }),
            playback: None,
        });

        if let Some(recording) = &self.active_streams[bank][button]
            && recording.recording.is_none()
        {
            bail!("Failed to initialise recording state");
        }

        Ok(())
    }

    pub fn stop_record(
        &mut self,
        bank: SampleBank,
        button: SampleButtons,
    ) -> Result<Option<(String, f64)>> {
        let mut file = None;

        if let Some(player) = &mut self.active_streams[bank][button] {
            if player.stream_type == StreamType::Playback {
                bail!("Attempted to Stop Recording on Playback Stream..");
            }

            if let Some(recording_state) = &mut player.recording {
                recording_state.state.stop.store(true, Ordering::Relaxed);
                recording_state.wait();

                debug!(
                    "Calculated Gain: {}",
                    recording_state.state.gain.load(Ordering::Relaxed)
                );

                // Recording Complete, check the file was made...
                if recording_state.file.exists() {
                    if let Some(file_name) = recording_state.file.file_name() {
                        let gain = recording_state.state.gain.load(Ordering::Relaxed);
                        file.replace((String::from(file_name.to_string_lossy()), gain));
                    } else {
                        bail!("Unable to Extract Filename from Path! (This shouldn't be possible!)")
                    }
                }
            }
        } else {
            bail!("Attempted to stop inactive recording..");
        }

        // Sample has been stopped, clear the state of this button.
        self.active_streams[bank][button] = None;
        Ok(file)
    }

    pub fn calculate_gain_thread(
        &mut self,
        path: PathBuf,
        bank: SampleBank,
        button: SampleButtons,
    ) -> Result<()> {
        if self.process_task.is_some() {
            bail!("Sample already being processed");
        }

        // Create the player..
        let mut player = Player::new(&path, None, None, None, None, None)?;

        // Grab the State..
        let state = player.get_state();

        // Spawn the Thread and Grab the Handler..
        let handler = thread::spawn(move || {
            player.calculate_gain();
        });

        // Store this into the processing task..
        self.process_task.replace(ProcessTask {
            bank,
            button,
            file: path,
            player: AudioPlaybackState {
                handle: Some(handler),
                state,
            },
        });

        Ok(())
    }

    pub fn is_calculating(&self) -> bool {
        self.process_task.is_some()
    }

    pub fn is_calculating_complete(&self) -> Result<bool> {
        if self.process_task.is_none() {
            bail!("Calculation not in progress");
        }

        if let Some(task) = &self.process_task {
            return Ok(task.player.is_finished());
        }
        bail!("Task exists, but also doesn't exist!");
    }

    pub fn get_calculating_progress(&self) -> Result<u8> {
        if self.process_task.is_none() {
            bail!("Calculation not in progress");
        }

        if let Some(task) = &self.process_task {
            return Ok(task.player.state.progress.load(Ordering::Relaxed));
        }

        bail!("Task exists, but also doesn't exist!");
    }

    pub fn get_and_clear_calculating_result(&mut self) -> Result<CalculationResult> {
        if self.process_task.is_none() {
            bail!("Calculation not in progress");
        }

        let result;
        if let Some(task) = &mut self.process_task {
            // We need to make sure the thread is finished..
            task.player.wait();

            let task_result = if let Ok(error) = task.player.state.error.lock() {
                if let Some(error) = error.as_ref() {
                    Err(anyhow!(error.clone()))
                } else {
                    Ok(())
                }
            } else {
                Err(anyhow!("Unable to read calculation state"))
            };

            result = CalculationResult {
                result: task_result,
                file: task.file.clone(),
                bank: task.bank,
                button: task.button,
                gain: task.player.state.calculated_gain.load(Ordering::Relaxed),
            };
        } else {
            bail!("Unable to obtain Task");
        }

        // In all cases, when we get here, we're done, so cleanup and go home
        self.process_task = None;
        Ok(result)
    }
}

impl Drop for AudioHandler {
    fn drop(&mut self) {
        if let Some(buffered_recorder) = &self.buffered_input {
            buffered_recorder.stop();
        }
    }
}

pub struct CalculationResult {
    pub result: Result<()>,
    pub file: PathBuf,
    pub bank: SampleBank,
    pub button: SampleButtons,
    pub gain: f64,
}
