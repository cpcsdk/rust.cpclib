"""
Test suite for bndbuild builders

This tests type-safe builder patterns for constructing bndbuild
commands without string manipulation. All parameters are validated at
compile time against the actual CLI structures.
"""

import unittest
import cpclib_python

# Alias for convenience
SnapshotBuilder = cpclib_python.bndbuild.SnapshotBuilder
ArkosTracker3Builder = cpclib_python.bndbuild.ArkosTracker3Builder
ChipnsfxBuilder = cpclib_python.bndbuild.ChipnsfxBuilder
MinyBuilder = cpclib_python.bndbuild.MinyBuilder
AytBuilder = cpclib_python.bndbuild.AytBuilder
SongConverterBuilder = cpclib_python.bndbuild.SongConverterBuilder


class TestSnapshotBuilder(unittest.TestCase):
    """Test SnapshotBuilder functionality"""

    def test_snapshot_builder_basic(self):
        """Test basic SnapshotBuilder construction"""
        task = (SnapshotBuilder()
                .in_snapshot("base.sna")
                .output("output.sna")
                .build())
        self.assertIsNotNone(task)

    def test_snapshot_builder_full(self):
        """Test SnapshotBuilder with all features"""
        task = (SnapshotBuilder()
                .in_snapshot("base.sna")
                .set_token("CRTC_REG:6", "20")
                .put_data(0x4000, 0xFF)
                .load("code.bin", "0x8000")
                .version("3")
                .output("output.sna")
                .build())
        self.assertIsNotNone(task)
    
    def test_snapshot_builder_chaining(self):
        """Test fluent API chaining"""
        builder = SnapshotBuilder()
        builder.in_snapshot("input.sna")
        builder.set_token("TOKEN", "value")
        builder.output("output.sna")
        task = builder.build()
        self.assertIsNotNone(task)


class TestTrackerBuilders(unittest.TestCase):
    """Test tracker builder functionality"""

    def test_arkos_tracker3_builder(self):
        """Test ArkosTracker3Builder"""
        task = (ArkosTracker3Builder()
                .input("song.aks")
                .output("song.bin")
                .build())
        self.assertIsNotNone(task)

    def test_chipnsfx_builder(self):
        """Test ChipnsfxBuilder"""
        task = (ChipnsfxBuilder()
                .input("effects.txt")
                .output("effects.bin")
                .build())
        self.assertIsNotNone(task)


class TestYmCruncherBuilders(unittest.TestCase):
    """Test YM cruncher builder functionality"""

    def test_miny_builder(self):
        """Test MinyBuilder"""
        task = (MinyBuilder()
                .input("song.ym")
                .output("song.min.ym")
                .build())
        self.assertIsNotNone(task)

    def test_ayt_builder(self):
        """Test AytBuilder"""
        task = (AytBuilder()
                .input("song.ym")
                .output("song.txt")
                .build())
        self.assertIsNotNone(task)


class TestSongConverterBuilder(unittest.TestCase):
    """Test SongConverterBuilder functionality"""

    def test_song_converter_ym(self):
        """Test SongConverter to YM format"""
        task = (SongConverterBuilder("ym")
                .input("song.aks")
                .output("song.ym")
                .build())
        self.assertIsNotNone(task)

    def test_song_converter_wav(self):
        """Test SongConverter to WAV format"""
        task = (SongConverterBuilder("wav")
                .input("song.aks")
                .output("song.wav")
                .build())
        self.assertIsNotNone(task)

    def test_song_converter_akm(self):
        """Test SongConverter to AKM format"""
        task = (SongConverterBuilder("akm")
                .input("song.aks")
                .output("song.akm")
                .build())
        self.assertIsNotNone(task)

    def test_song_converter_vgm(self):
        """Test SongConverter to VGM format"""
        task = (SongConverterBuilder("vgm")
                .input("song.aks")
                .output("song.vgm")
                .build())
        self.assertIsNotNone(task)

    def test_song_converter_soundeffects(self):
        """Test SongConverter to sound effects"""
        task = (SongConverterBuilder("soundeffects")
                .input("song.aks")
                .output("effects.bin")
                .build())
        self.assertIsNotNone(task)
    
    def test_song_converter_invalid_type(self):
        """Test SongConverter with invalid type raises error"""
        with self.assertRaises(ValueError):
            (SongConverterBuilder("invalid")
             .input("song.aks")
             .output("output")
             .build())
    
    def test_song_converter_all_types(self):
        """Test all valid converter types"""
        valid_types = ["akg", "akm", "aky", "events", "raw", 
                      "soundeffects", "vgm", "wav", "ym", "z80profiler"]
        for conv_type in valid_types:
            with self.subTest(type=conv_type):
                task = (SongConverterBuilder(conv_type)
                        .input("song.aks")
                        .output("output")
                        .build())
                self.assertIsNotNone(task)


if __name__ == "__main__":
    unittest.main()
