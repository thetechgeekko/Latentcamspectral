# Design Blueprint: Latent Cam 2.0

Since you are a photographer, you already know that a camera isn't just a tool; it's an instrument. Apps like *mood.camera* succeed because they feel tactile, opinionated, and evoke the nostalgia of analog photography. We are going to build a Jetpack Compose UI that has **soul**. 

We will avoid standard Android Material Design (which looks generic and corporate) and build a custom, minimalist, tactile interface.

## 1. Visual Language & Aesthetics

- **The Color Palette:** True Black (`#000000`), Dark Gray (`#1A1A1A`), and striking accent colors (like a deep Leica Red or a subtle Kodak Gold) for active states.
- **Typography:** We will mix a technical Sans-Serif (like *Inter* or *Roboto Mono*) for camera metadata (ISO, Shutter, EV) with a classic, elegant Serif (like *Playfair Display* or *Libre Baskerville*) for the Film Stock names.
- **Tactility:** Every dial and button press will be wired to Android's `HapticFeedback`. We want it to feel like you are clicking physical metal dials.

## 2. The Rangefinder Viewfinder

Instead of a generic full-screen edge-to-edge display, we will design a restricted, intentional **Rangefinder-style** layout.
- **The Viewport:** A distinct, framed rectangle in the upper-center of the screen. Crucially, it will feature **smooth, rounded edges** (like looking through beveled glass or a high-end modern UI). It forces intentional framing, just like looking through a physical camera.
- **The Shutter Button:** Placed ergonomically in the lower half of the dark interface. A large, textured, mechanical-looking trigger.
- **The Dials:** Beautiful, scrolling "Knurled Dials" for Exposure Compensation (-2 to +2) and Focus that click with haptics as you spin them under the viewport.
- **The Film Selector:** A subtle carousel or dial at the bottom. When you switch from *Portra* to *Cinestill*, the name smoothly transitions, the engine's 3D LUT updates instantly, and the framed viewfinder colors shift in real-time.

## 3. The "Developing" Darkroom Experience

When you hit the shutter, we don't just dump a file into the gallery.
- A notification drops into the corner showing a "Roll of Film" icon.
- It displays a subtle, glowing "Developing..." animation while the Rust engine crunches the high-res 12MP math (Halation, Grain, Spectral Density).
- Once finished, it slides into a beautiful "Print Gallery."

## 4. Technical App Structure

To execute this, we will set up the project as follows:
- **Location:** I recommend building this as an `app` module directly inside the current `Spectralfilmengine` workspace so we can easily iterate on the Rust engine and the Kotlin UI simultaneously.
- **Framework:** 100% Jetpack Compose for the UI.
- **Camera API:** Camera2 for true `RAW_SENSOR` access.

## User Review Required

I have added the rounded edges to the Viewfinder section! If you are happy with this final blueprint, let's execute!
