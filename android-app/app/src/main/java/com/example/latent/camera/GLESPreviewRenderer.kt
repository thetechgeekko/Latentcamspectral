package com.example.latent.camera

import android.graphics.SurfaceTexture
import android.opengl.GLES11Ext
import android.opengl.GLES20
import android.opengl.GLES30
import android.opengl.GLUtils
import android.util.Log
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer

/**
 * OpenGL ES 3.0 Renderer that applies a 3D LUT to the camera preview.
 * This gives us real-time film emulation in the viewfinder.
 */
class GLESPreviewRenderer {

    companion object {
        private const val TAG = "GLESPreviewRenderer"

        private const val VERTEX_SHADER = """#version 300 es
            in vec4 aPosition;
            in vec2 aTexCoord;
            uniform mat4 uTexMatrix;
            out vec2 vTexCoord;
            void main() {
                gl_Position = aPosition;
                vTexCoord = (uTexMatrix * vec4(aTexCoord, 0.0, 1.0)).xy;
            }
        """

        private const val FRAGMENT_SHADER = """#version 300 es
            #extension GL_OES_EGL_image_external_essl3 : require
            precision mediump float;
            in vec2 vTexCoord;
            uniform samplerExternalOES sTexture;
            uniform mediump sampler3D sLut;
            uniform bool uLutEnabled;
            out vec4 fragColor;

            void main() {
                vec4 color = texture(sTexture, vTexCoord);
                if (uLutEnabled) {
                    // Camera preview is gamma-encoded (~sRGB); decode to linear for LUT lookup.
                    // LUT was built in linear light. Re-encode output to sRGB for display.
                    vec3 linear = pow(max(color.rgb, vec3(0.0)), vec3(2.2));
                    vec3 film   = pow(max(texture(sLut, linear).rgb, vec3(0.0)), vec3(1.0 / 2.2));
                    fragColor = vec4(film, color.a);
                } else {
                    fragColor = color;
                }
            }
        """
    }

    private var program = 0
    private var textureId = -1
    private var lutTextureId = -1
    private var vertexBuffer: FloatBuffer? = null
    private var texCoordBuffer: FloatBuffer? = null

    private var lutEnabled = false

    var isInitialized = false
        private set

    fun init() {
        try {
            program = createProgram(VERTEX_SHADER, FRAGMENT_SHADER)

            val textures = IntArray(1)
            GLES20.glGenTextures(1, textures, 0)
            textureId = textures[0]
            GLES20.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, textureId)
            GLES20.glTexParameterf(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_LINEAR.toFloat())
            GLES20.glTexParameterf(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MAG_FILTER, GLES20.GL_LINEAR.toFloat())
            GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_WRAP_S, GLES20.GL_CLAMP_TO_EDGE)
            GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_WRAP_T, GLES20.GL_CLAMP_TO_EDGE)

            // Vertex data (Full screen quad)
            val vData = floatArrayOf(
                -1f, -1f, 1f, -1f, -1f, 1f, 1f, 1f
            )
            vertexBuffer = ByteBuffer.allocateDirect(vData.size * 4).order(ByteOrder.nativeOrder()).asFloatBuffer().put(vData)
            vertexBuffer?.position(0)

            // Texture coordinates
            val tData = floatArrayOf(
                0f, 0f, 1f, 0f, 0f, 1f, 1f, 1f
            )
            texCoordBuffer = ByteBuffer.allocateDirect(tData.size * 4).order(ByteOrder.nativeOrder()).asFloatBuffer().put(tData)
            texCoordBuffer?.position(0)

            isInitialized = true
        } catch (e: Exception) {
            Log.e(TAG, "GL init failed", e)
        }
    }

    fun getTextureId() = textureId

    /**
     * Uploads a 3D LUT to the GPU.
     * @param lutData Interleaved RGB float data (size: size^3 * 3)
     * @param size Typically 33
     */
    fun updateLut(lutData: FloatArray, size: Int) {
        if (!isInitialized) return
        if (lutTextureId != -1) {
            val textures = intArrayOf(lutTextureId)
            GLES20.glDeleteTextures(1, textures, 0)
        }

        val textures = IntArray(1)
        GLES30.glGenTextures(1, textures, 0)
        lutTextureId = textures[0]

        GLES30.glBindTexture(GLES30.GL_TEXTURE_3D, lutTextureId)
        GLES30.glTexParameteri(GLES30.GL_TEXTURE_3D, GLES30.GL_TEXTURE_MIN_FILTER, GLES30.GL_LINEAR)
        GLES30.glTexParameteri(GLES30.GL_TEXTURE_3D, GLES30.GL_TEXTURE_MAG_FILTER, GLES30.GL_LINEAR)
        GLES30.glTexParameteri(GLES30.GL_TEXTURE_3D, GLES30.GL_TEXTURE_WRAP_S, GLES30.GL_CLAMP_TO_EDGE)
        GLES30.glTexParameteri(GLES30.GL_TEXTURE_3D, GLES30.GL_TEXTURE_WRAP_T, GLES30.GL_CLAMP_TO_EDGE)
        GLES30.glTexParameteri(GLES30.GL_TEXTURE_3D, GLES30.GL_TEXTURE_WRAP_R, GLES30.GL_CLAMP_TO_EDGE)

        val buffer = ByteBuffer.allocateDirect(lutData.size * 4).order(ByteOrder.nativeOrder()).asFloatBuffer().put(lutData)
        buffer.position(0)

        GLES30.glTexImage3D(
            GLES30.GL_TEXTURE_3D, 0, GLES30.GL_RGB32F,
            size, size, size, 0, GLES30.GL_RGB, GLES30.GL_FLOAT, buffer
        )
        
        lutEnabled = true
        Log.i(TAG, "Uploaded 3D LUT to GPU (size $size)")
    }

    fun draw(st: SurfaceTexture) {
        if (!isInitialized) return
        GLES20.glClearColor(0f, 0f, 0f, 1f)
        GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)

        GLES20.glUseProgram(program)

        // Upload SurfaceTexture transform matrix — corrects camera sensor rotation
        val texMatrix = FloatArray(16)
        st.getTransformMatrix(texMatrix)
        GLES20.glUniformMatrix4fv(GLES20.glGetUniformLocation(program, "uTexMatrix"), 1, false, texMatrix, 0)

        val ph = GLES20.glGetAttribLocation(program, "aPosition")
        GLES20.glEnableVertexAttribArray(ph)
        GLES20.glVertexAttribPointer(ph, 2, GLES20.GL_FLOAT, false, 0, vertexBuffer)

        val th = GLES20.glGetAttribLocation(program, "aTexCoord")
        GLES20.glEnableVertexAttribArray(th)
        GLES20.glVertexAttribPointer(th, 2, GLES20.GL_FLOAT, false, 0, texCoordBuffer)

        // Bind Camera Texture
        GLES20.glActiveTexture(GLES20.GL_TEXTURE0)
        GLES20.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, textureId)
        GLES20.glUniform1i(GLES20.glGetUniformLocation(program, "sTexture"), 0)

        // Bind 3D LUT
        if (lutEnabled && lutTextureId != -1) {
            GLES30.glActiveTexture(GLES30.GL_TEXTURE1)
            GLES30.glBindTexture(GLES30.GL_TEXTURE_3D, lutTextureId)
            GLES30.glUniform1i(GLES30.glGetUniformLocation(program, "sLut"), 1)
            GLES30.glUniform1i(GLES30.glGetUniformLocation(program, "uLutEnabled"), 1)
        } else {
            GLES30.glUniform1i(GLES30.glGetUniformLocation(program, "uLutEnabled"), 0)
        }

        GLES20.glDrawArrays(GLES20.GL_TRIANGLE_STRIP, 0, 4)
    }

    private fun createProgram(vertexSource: String, fragmentSource: String): Int {
        val vs = loadShader(GLES20.GL_VERTEX_SHADER, vertexSource)
        val fs = loadShader(GLES20.GL_FRAGMENT_SHADER, fragmentSource)
        val prog = GLES20.glCreateProgram()
        GLES20.glAttachShader(prog, vs)
        GLES20.glAttachShader(prog, fs)
        GLES20.glLinkProgram(prog)
        return prog
    }

    private fun loadShader(type: Int, source: String): Int {
        val shader = GLES20.glCreateShader(type)
        GLES20.glShaderSource(shader, source)
        GLES20.glCompileShader(shader)
        return shader
    }

    fun release() {
        if (program != 0) GLES20.glDeleteProgram(program)
        if (textureId != -1) GLES20.glDeleteTextures(1, intArrayOf(textureId), 0)
        if (lutTextureId != -1) GLES20.glDeleteTextures(1, intArrayOf(lutTextureId), 0)
    }
}
