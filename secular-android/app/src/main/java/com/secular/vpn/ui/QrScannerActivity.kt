// secular-android/app/src/main/java/com/secular/vpn/ui/QrScannerActivity.kt
// Custom QR scanner — with green frame overlay + animated scan line

package com.secular.vpn.ui

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.animation.Animation
import android.view.animation.TranslateAnimation
import android.widget.ImageView
import com.google.zxing.ResultPoint
import com.journeyapps.barcodescanner.BarcodeCallback
import com.journeyapps.barcodescanner.BarcodeResult
import com.journeyapps.barcodescanner.DecoratedBarcodeView
import com.secular.vpn.R

class QrScannerActivity : Activity() {

    private lateinit var barcodeScannerView: DecoratedBarcodeView
    private lateinit var scanLine: View
    private val handler = Handler(Looper.getMainLooper())
    private var isDone = false

    private val callback = BarcodeCallback { result ->
        if (isDone) return@BarcodeCallback
        val text = result.text
        if (text != null && text.isNotEmpty()) {
            isDone = true
            SecularVpnService.addLog("QR scanned: ${text.take(80)}")
            val resultIntent = Intent().putExtra(QR_RESULT, text)
            setResult(RESULT_OK, resultIntent)
            finish()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_qr_scanner)

        barcodeScannerView = findViewById(R.id.barcode_scanner)
        scanLine = findViewById(R.id.scanner_line)

        // Close button
        findViewById<ImageView>(R.id.scanner_close).setOnClickListener {
            setResult(RESULT_CANCELED)
            finish()
        }
    }

    override fun onResume() {
        super.onResume()
        barcodeScannerView.decodeContinuous(callback)
        startScanAnimation()
    }

    override fun onPause() {
        super.onPause()
        barcodeScannerView.pause()
        scanLine.clearAnimation()
    }

    private fun startScanAnimation() {
        val anim = TranslateAnimation(
            Animation.RELATIVE_TO_PARENT, 0f,
            Animation.RELATIVE_TO_PARENT, 0f,
            Animation.RELATIVE_TO_PARENT, -0.46f,
            Animation.RELATIVE_TO_PARENT, 0.46f
        )
        anim.duration = 2000
        anim.repeatCount = Animation.INFINITE
        anim.repeatMode = Animation.REVERSE
        scanLine.startAnimation(anim)
    }

    companion object {
        const val QR_RESULT = "qr_result"
    }
}
