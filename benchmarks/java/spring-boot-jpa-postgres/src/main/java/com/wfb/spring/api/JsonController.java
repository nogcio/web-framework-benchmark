package com.wfb.spring.api;

import com.wfb.spring.api.dto.WebAppPayload;
import com.wfb.spring.api.service.JsonReplaceService;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

@RestController
public class JsonController {
    private final JsonReplaceService replaceService;

    public JsonController(JsonReplaceService replaceService) {
        this.replaceService = replaceService;
    }

    @PostMapping("/json/{from}/{to}")
    public WebAppPayload json(@PathVariable String from, @PathVariable String to, @RequestBody WebAppPayload body) {
        return replaceService.replaceServletName(body, from, to);
    }
}
