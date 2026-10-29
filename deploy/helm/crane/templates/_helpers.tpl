{{- define "crane.fullname" -}}
{{- printf "%s-%s" .Release.Name .Chart.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "crane.image" -}}
{{- $ref := printf "%s:%s" .Values.image.repository (default .Chart.AppVersion .Values.image.tag) -}}
{{- if .Values.image.digest -}}{{ printf "%s@%s" $ref .Values.image.digest }}{{- else -}}{{ $ref }}{{- end -}}
{{- end -}}

{{- define "crane.labels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version }}
{{- end -}}

{{- define "crane.selector" -}}
app.kubernetes.io/name: {{ .root.Chart.Name }}
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .component }}
{{- end -}}

{{/* Общая часть пода: образ, окружение и жёсткий securityContext (restricted PSS). */}}
{{- define "crane.podspec" -}}
{{- with .root.Values.imagePullSecrets }}
imagePullSecrets: {{ toYaml . | nindent 2 }}
{{- end }}
securityContext:
  runAsNonRoot: true
  seccompProfile: { type: RuntimeDefault }
containers:
  - name: {{ .component }}
    image: {{ include "crane.image" .root }}
    imagePullPolicy: {{ .root.Values.image.pullPolicy }}
    args: [{{ .role | quote }}]
    envFrom:
      - secretRef:
          name: {{ required "existingSecret обязателен" .root.Values.existingSecret }}
    env:
      {{- with .root.Values.config.cache }}
      - name: CRANE_CACHE
        value: {{ . | quote }}
      {{- end }}
      {{- with .root.Values.config.blobs }}
      - name: CRANE_BLOBS
        value: {{ . | quote }}
      {{- end }}
      {{- if and (eq .role "serve") .root.Values.bootstrapToken.secretName }}
      - name: CRANE_BOOTSTRAP_TOKEN_FILE
        value: {{ printf "/run/secrets/bootstrap/%s" .root.Values.bootstrapToken.key | quote }}
      {{- end }}
    securityContext:
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities: { drop: [ALL] }
    volumeMounts:
      - { name: tmp, mountPath: /tmp }
      {{- if and (eq .role "serve") .root.Values.bootstrapToken.secretName }}
      - { name: bootstrap, mountPath: /run/secrets/bootstrap, readOnly: true }
      {{- end }}
volumes:
  - { name: tmp, emptyDir: {} }
  {{- if and (eq .role "serve") .root.Values.bootstrapToken.secretName }}
  - name: bootstrap
    secret:
      secretName: {{ .root.Values.bootstrapToken.secretName | quote }}
      # 0400 в восьмеричной записи: читает только пользователь пода.
      defaultMode: 256
  {{- end }}
{{- end -}}
