<?php

use Illuminate\Support\Facades\Route;
use App\Http\Controllers\BenchmarkController;

Route::get('/health', function () {
    try {
        \Illuminate\Support\Facades\DB::select('SELECT 1');
        return response('OK');
    } catch (\Throwable $e) {
        error_log($e->getMessage());
        return response('Database Error', 500);
    }
});

Route::get('/plaintext', [BenchmarkController::class, 'plaintext']);
Route::post('/json/aggregate', [BenchmarkController::class, 'json']);
Route::get('/db/user-profile/{email}', [BenchmarkController::class, 'dbUserProfile']);
