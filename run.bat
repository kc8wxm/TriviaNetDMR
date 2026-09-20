@echo off
title TriviaNetDMR - Net Control Companion
cls

:: ============================================================
:: QRZ.com Live Lookup Credentials (Optional)
:: If left commented out (::), TriviaNetDMR runs in Offline
:: Mock Mode, automatically generating realistic operator data.
:: ============================================================
:: set QRZ_USERNAME=your_callsign
:: set QRZ_PASSWORD=your_password

echo Starting TriviaNetDMR...
TriviaNetDMR.exe
