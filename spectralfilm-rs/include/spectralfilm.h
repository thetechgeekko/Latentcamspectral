#pragma once

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Engine SpectralFilmEngine;

/** Create an engine from two profile JSON strings. Returns NULL on failure. */
SpectralFilmEngine* spectralfilm_create_from_json(const char* film_json, const char* print_json);

/** Destroy an engine created by spectralfilm_create_from_json. */
void spectralfilm_destroy(SpectralFilmEngine* engine);

/**
 * Process interleaved linear RGB float input.
 *
 * input length must be width*height*3.
 * output capacity must be large enough for the returned image.
 * Return value is the number of floats written, or 0 on error.
 */
size_t spectralfilm_process_f32(
    SpectralFilmEngine* engine,
    const float* input,
    size_t width,
    size_t height,
    float* output,
    size_t output_capacity
);

/** Dimensions of the last processed output image. */
size_t spectralfilm_last_width(const SpectralFilmEngine* engine);
size_t spectralfilm_last_height(const SpectralFilmEngine* engine);

/** Last error string. Caller must free with spectralfilm_free_string. */
char* spectralfilm_last_error(const SpectralFilmEngine* engine);
void spectralfilm_free_string(char* s);

#ifdef __cplusplus
}
#endif
