package com.wfb.spring.api.dto;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

@JsonInclude(JsonInclude.Include.NON_NULL)
public class Taglib {
    @JsonProperty("taglib-uri")
    private String taglibUri;

    @JsonProperty("taglib-location")
    private String taglibLocation;

    public String getTaglibUri() {
        return taglibUri;
    }

    public void setTaglibUri(String taglibUri) {
        this.taglibUri = taglibUri;
    }

    public String getTaglibLocation() {
        return taglibLocation;
    }

    public void setTaglibLocation(String taglibLocation) {
        this.taglibLocation = taglibLocation;
    }
}
