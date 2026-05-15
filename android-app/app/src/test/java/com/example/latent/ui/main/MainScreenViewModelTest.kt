package com.example.latent.ui.main

import com.example.latent.processing.FILM_STOCKS
import junit.framework.TestCase.assertEquals
import junit.framework.TestCase.assertTrue
import org.junit.Test

class FilmStocksTest {

    @Test
    fun filmStocks_allHaveNonEmptyProfileFilenames() {
        FILM_STOCKS.forEach { stock ->
            assertTrue(
                "${stock.displayName}: filmProfile must not be empty",
                stock.filmProfile.isNotEmpty()
            )
            assertTrue(
                "${stock.displayName}: printProfile must not be empty",
                stock.printProfile.isNotEmpty()
            )
        }
    }

    @Test
    fun filmStocks_noDuplicateDisplayNames() {
        val names = FILM_STOCKS.map { it.displayName }
        assertEquals(
            "Duplicate display names: ${names.groupBy { it }.filter { it.value.size > 1 }.keys}",
            names.size,
            names.toSet().size
        )
    }

    @Test
    fun filmStocks_allProfileFilenamesEndWithJson() {
        FILM_STOCKS.forEach { stock ->
            assertTrue("${stock.filmProfile} must end with .json", stock.filmProfile.endsWith(".json"))
            assertTrue("${stock.printProfile} must end with .json", stock.printProfile.endsWith(".json"))
        }
    }

    @Test
    fun filmStocks_minimumLibrarySize() {
        assertTrue("Expected at least 20 film stocks, got ${FILM_STOCKS.size}", FILM_STOCKS.size >= 20)
    }
}
